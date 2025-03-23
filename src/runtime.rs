use crate::{
    config::{self, Config, SettingMode, VStr},
    iio::sensor_proxy::{AccelerometerOrientation, SensorProxyProxy},
    libinput::{EventListener, new_libinput},
};
use anyhow::{Context, Result, anyhow};
use futures::StreamExt;
use input::{
    Device, Event,
    event::{
        SwitchEvent,
        switch::{Switch, SwitchState},
    },
};
use std::{
    collections::btree_map::{BTreeMap, Entry},
    num::NonZeroUsize,
};
use tokio::{
    process::Command,
    select,
    sync::{mpsc, watch},
};
use zbus::Connection;

const DEFAULT_ROTATION: [f32; 6] = [1., 0., 0., 0., 1., 0.];
const ROTATE_90: [f32; 6] = [0., -1., 1., 1., 0., 0.];
const ROTATE_180: [f32; 6] = [-1., 0., 1., 0., -1., 1.];
const ROTATE_270: [f32; 6] = [0., 1., 0., -1., 0., 1.];

#[derive(Debug)]
pub struct Runtime {
    on_mode_laptop: Option<ActionId>,
    on_mode_tablet: Option<ActionId>,
    on_rotate_normal: Option<ActionId>,
    on_rotate_left_up: Option<ActionId>,
    on_rotate_right_up: Option<ActionId>,
    on_rotate_bottom_up: Option<ActionId>,
    actions: Vec<Action>,
    event: EventListener,
    touchscreen: Option<Device>,
    default_mode: SettingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionId(NonZeroUsize);

impl From<ActionId> for usize {
    fn from(value: ActionId) -> Self {
        value.0.get() - 1
    }
}

impl From<usize> for ActionId {
    fn from(value: usize) -> Self {
        Self(NonZeroUsize::new(value + 1).unwrap())
    }
}

type Action = Vec<Task>;

#[derive(Debug)]
enum Task {
    Action(ActionId),
    Cmd(Vec<String>),
    RotateCalibration(RotationMode),
    Rotation(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RotationMode {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl Runtime {
    pub fn new(con: Config) -> Result<Self> {
        let mut helper = RuntimeHelper::new(con);
        let on_mode_laptop = helper.get_action("on-mode-laptop").transpose()?;
        let on_mode_tablet = helper.get_action("on-mode-tablet").transpose()?;
        let on_rotate_normal = helper.get_action("on-rotate-normal").transpose()?;
        let on_rotate_left_up = helper.get_action("on-rotate-left-up").transpose()?;
        let on_rotate_right_up = helper.get_action("on-rotate-right-up").transpose()?;
        let on_rotate_bottom_up = helper.get_action("on-rotate-bottom-up").transpose()?;
        let touchscreen = helper
            .settings
            .touchscreen
            .map(|path| new_libinput().path_add_device(path.as_str()))
            .flatten();
        let mut event = EventListener::new()?;
        event
            .path_add_device(&helper.settings.switch)
            .context("Cannot add switch")?;

        Ok(Self {
            on_mode_laptop,
            on_mode_tablet,
            on_rotate_normal,
            on_rotate_left_up,
            on_rotate_right_up,
            on_rotate_bottom_up,
            event,
            touchscreen,
            actions: helper.runtime_actions,
            default_mode: helper.settings.default_mode,
        })
    }
    pub async fn run(mut self) -> Result<()> {
        let (rotation, mut rotation_r) = watch::channel(false);
        let (rotation_calibration, mut rotation_calibration_r) =
            watch::channel(RotationMode::Normal);
        let (action, mut action_r) = mpsc::unbounded_channel();

        let action_rt = ActionRuntime {
            action: action.clone(),
            rotation,
            rotation_calibration,
        };

        match self.default_mode {
            SettingMode::Laptop => self.on_mode_laptop.map(|id| action.send(id)).transpose(),
            SettingMode::Tablet => self.on_mode_tablet.map(|id| action.send(id)).transpose(),
        }?;

        let conn = Connection::system().await?;
        let proxy = SensorProxyProxy::new(&conn).await?;
        let mut accelerometer = proxy.receive_accelerometer_orientation_changed().await;

        // always have the init vaule
        // we don't need that
        let _ = accelerometer.next().await;

        loop {
            select! {
                // action queue
                id = action_r.recv() => {
                    let id = usize::from(id.context("Cannot receive action")?);
                    let action = &self.actions[id];
                    log::debug!("Running action: {id}, {:?}", action);

                    action_rt.run_action(action).await?;
                }

                // enable/disable rotation
                res = rotation_r.changed() => {
                    res?;

                    match *rotation_r.borrow() {
                        true => {
                            log::info!("Enable rotation");
                            proxy.claim_accelerometer().await?;
                        }
                        false => {
                            log::info!("Disable rotation");
                            proxy.release_accelerometer().await?
                        }
                    }
                }

                res = rotation_calibration_r.changed() => {
                    res?;

                    let calibration = rotation_calibration_r.borrow();
                    let Some(touchscreen) = &mut self.touchscreen else {
                        continue;
                    };
                    let normal = touchscreen
                        .config_calibration_default_matrix()
                        .unwrap_or(DEFAULT_ROTATION);
                    let matrix = match *calibration {
                        RotationMode::Normal => normal,
                        RotationMode::Rotate90 => calibration_matrix_product(normal, ROTATE_90),
                        RotationMode::Rotate180 => calibration_matrix_product(normal, ROTATE_180),
                        RotationMode::Rotate270 => calibration_matrix_product(normal, ROTATE_270),
                    };
                    log::info!("Set calibration to: {:?}", matrix);
                    touchscreen.config_calibration_set_matrix(matrix)
                        .map_err(|err| anyhow!("Set calibration matrix error: {:?}", err))?;
                }

                // libinput event
                Some(event) = self.event.next() => {
                    let event = event?;
                    match event {
                        Event::Device(dev) => log::info!("Device event: {:?}", dev),

                        Event::Switch(SwitchEvent::Toggle(event)) => {
                            if Some(Switch::TabletMode) != event.switch() {
                                log::info!("Get non-tablet switch event, discard");
                                continue;
                            }
                            match event.switch_state() {
                                SwitchState::On => {
                                    log::info!("Enter tablet mode");
                                    self.on_mode_tablet.map(|id| action.send(id)).transpose()?;
                                }
                                SwitchState::Off => {
                                    log::info!("Enter laptop mode");
                                    self.on_mode_laptop.map(|id| action.send(id)).transpose()?;
                                }
                            }
                        }

                        event => log::warn!("Unknown event: {:?}", event),
                    }
                }

                // accelerometer
                Some(event) = accelerometer.next() => {
                    let event = event.get().await?;
                    match event {
                        AccelerometerOrientation::Normal => {
                            self.on_rotate_normal
                                .map(|id| action.send(id))
                                .transpose()?;
                        }
                        AccelerometerOrientation::BottomUp => {
                            self.on_rotate_bottom_up
                                .map(|id| action.send(id))
                                .transpose()?;
                        }
                        AccelerometerOrientation::LeftUp => {
                            self.on_rotate_left_up
                                .map(|id| action.send(id))
                                .transpose()?;
                        }
                        AccelerometerOrientation::RightUp => {
                            self.on_rotate_right_up
                                .map(|id| action.send(id))
                                .transpose()?;
                        }
                        AccelerometerOrientation::Undefined => log::warn!("Undefined rotation"),
                        AccelerometerOrientation::Unknown(value) => {
                            log::error!("Unknown rotation: {value}")
                        }
                    }
                }


            }
        }
    }
}

/// matrix is look like this
///
/// ```text
/// [0] [1] [2]
/// [3] [4] [5]
///  0   0   1
/// ```
fn calibration_matrix_product(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[3],
        a[0] * b[1] + a[1] * b[4],
        a[0] * b[2] + a[1] * b[5] + a[2],
        a[3] * b[0] + a[4] * b[3],
        a[3] * b[1] + a[4] * b[4],
        a[3] * b[2] + a[4] * b[5] + a[5],
    ]
}

struct ActionRuntime {
    action: mpsc::UnboundedSender<ActionId>,
    rotation: watch::Sender<bool>,
    rotation_calibration: watch::Sender<RotationMode>,
}

impl ActionRuntime {
    async fn run_action(&self, action: &Action) -> Result<()> {
        for task in action {
            self.run_task(task).await?;
        }
        Ok(())
    }

    async fn run_task(&self, task: &Task) -> Result<()> {
        log::debug!("Running task: {:?}", task);
        match task {
            Task::Action(id) => self.action.send(*id)?,
            Task::Cmd(args) => {
                log::info!("Running command: {:?}", args);
                let mut cmd =
                    Command::new(args.first().context("cmd should had at least one args")?);
                cmd.args(&args[1..]);
                cmd.spawn()?.wait().await?;
            }
            Task::Rotation(enable) => {
                self.rotation.send_if_modified(|old| {
                    let change = old != enable;
                    *old = *enable;
                    change
                });
            }
            Task::RotateCalibration(mode) => {
                self.rotation_calibration.send_if_modified(|old| {
                    let change = old != mode;
                    *old = *mode;
                    change
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct RuntimeHelper {
    actions: BTreeMap<String, Vec<config::Task>>,
    variables: BTreeMap<String, config::VStr>,
    settings: config::Settings,
    action_id_map: BTreeMap<String, ActionId>,
    runtime_actions: Vec<Action>,
}

impl RuntimeHelper {
    fn new(con: Config) -> Self {
        let actions = con
            .actions
            .into_iter()
            .map(|x| x.actions.into_iter().map(|y| (y.event, y.tasks)))
            .flatten()
            .collect();
        let variables = con
            .varibles
            .into_iter()
            .map(|x| x.variables.into_iter().map(|y| (y.name, y.value)))
            .flatten()
            .collect();
        Self {
            actions,
            variables,
            settings: con.settings,
            action_id_map: Default::default(),
            runtime_actions: Default::default(),
        }
    }

    fn get_action(&mut self, s: impl Into<String>) -> Option<Result<ActionId>> {
        let s: String = s.into();
        let id = match self.action_id_map.entry(s) {
            Entry::Vacant(vacant_entry) => {
                let id = ActionId::from(self.runtime_actions.len());

                // placeholder
                self.runtime_actions.push(vec![]);

                let tasks = self.actions.remove(vacant_entry.key())?;
                vacant_entry.insert(id);

                let tasks = tasks
                    .into_iter()
                    .map(|s| self.resolve_task(s))
                    .collect::<Result<Vec<_>>>();
                match tasks {
                    Ok(tasks) => self.runtime_actions[<_ as Into<usize>>::into(id)] = tasks,
                    Err(e) => return Some(Err(e)),
                };

                id
            }
            Entry::Occupied(occupied_entry) => *occupied_entry.get(),
        };

        Some(Ok(id))
    }

    fn get_variable(&self, s: impl Into<String>) -> Result<String> {
        let mut s = s.into();
        loop {
            let variable = self
                .variables
                .get(&s)
                .ok_or_else(|| anyhow!("Cannot find variable: {s}"))?;
            match variable {
                VStr::Value(v) => return Ok(v.clone()),
                VStr::Config(c) => s = c.clone(),
            }
        }
    }

    fn resolve_vstr(&self, s: VStr) -> Result<String> {
        match s {
            VStr::Value(v) => Ok(v),
            VStr::Config(c) => self.get_variable(c),
        }
    }

    fn resolve_task(&mut self, task: config::Task) -> Result<Task> {
        let task = match task {
            config::Task::Action(s) => {
                let name = self.resolve_vstr(s)?;
                Task::Action(
                    self.get_action(name.clone())
                        .ok_or_else(|| anyhow!("Cannot resolve action {name}"))??,
                )
            }
            config::Task::Cmd(ss) => Task::Cmd(
                ss.into_iter()
                    .map(|s| self.resolve_vstr(s))
                    .collect::<Result<Vec<_>>>()?,
            ),
            config::Task::Rotation(s) => Task::Rotation(match self.resolve_vstr(s)?.as_str() {
                "enable" => true,
                "disable" => false,
                s => return Err(anyhow!("Unknown vaule for rotation: {s}")),
            }),
            config::Task::RotateCalibration(s) => {
                Task::RotateCalibration(match self.resolve_vstr(s)?.as_str() {
                    "normal" => RotationMode::Normal,
                    "rotate90" => RotationMode::Rotate90,
                    "rotate180" => RotationMode::Rotate180,
                    "rotate270" => RotationMode::Rotate270,
                    s => return Err(anyhow!("Uknown value for rotate calibration: {s}")),
                })
            }
        };
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_matrix() {
        assert_eq!(
            calibration_matrix_product(DEFAULT_ROTATION, DEFAULT_ROTATION),
            DEFAULT_ROTATION
        );
        assert_eq!(
            calibration_matrix_product(DEFAULT_ROTATION, ROTATE_90),
            ROTATE_90
        );
        assert_eq!(
            calibration_matrix_product(DEFAULT_ROTATION, ROTATE_180),
            ROTATE_180
        );
        assert_eq!(
            calibration_matrix_product(DEFAULT_ROTATION, ROTATE_270),
            ROTATE_270
        );
    }
}
