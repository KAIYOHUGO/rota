use anyhow::Result;
use futures::Stream;
use input::{Event, Libinput, LibinputInterface};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use std::{
    fs::{File, OpenOptions},
    ops::{Deref, DerefMut},
    os::unix::{fs::OpenOptionsExt, io::OwnedFd},
    path::Path,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tokio::io::{Interest, unix::AsyncFd};

/// a event listener libinput async wrapper
#[derive(Debug)]
pub struct EventListener(AsyncFd<Libinput>, ListenerState);

#[derive(Debug, Default)]
enum ListenerState {
    #[default]
    Fd,
    Iter,
}

/// create a new path libinput
pub fn new_libinput() -> Libinput {
    Libinput::new_from_path(Interface)
}

impl EventListener {
    pub fn new() -> Result<Self> {
        let input = AsyncFd::with_interest(new_libinput(), Interest::READABLE)?;
        Ok(Self(input, Default::default()))
    }
}

impl Deref for EventListener {
    type Target = Libinput;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

impl DerefMut for EventListener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut()
    }
}

impl Stream for EventListener {
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.1 {
            ListenerState::Fd => {
                let mut guard = match ready!(self.0.poll_read_ready_mut(cx)) {
                    Ok(guard) => guard,
                    Err(err) => return Poll::Ready(Some(Err(err.into()))),
                };
                if let Err(e) = guard.get_inner_mut().dispatch() {
                    return Poll::Ready(Some(Err(e.into())));
                }
                guard.clear_ready();

                self.1 = ListenerState::Iter;
                self.poll_next(cx)
            }

            ListenerState::Iter => match self.0.get_mut().next() {
                Some(event) => Poll::Ready(Some(Ok(event))),
                None => {
                    self.1 = ListenerState::Fd;
                    self.poll_next(cx)
                }
            },
        }
    }
}

struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        let _ = File::from(fd);
    }
}
