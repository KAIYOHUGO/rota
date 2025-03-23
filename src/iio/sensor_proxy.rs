use zbus::{
    proxy,
    zvariant::{self, OwnedValue, Type, Value},
    Result,
};

#[proxy(
    interface = "net.hadess.SensorProxy",
    default_service = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy"
)]
trait SensorProxy {
    /// ClaimAccelerometer method
    fn claim_accelerometer(&self) -> Result<()>;

    /// ClaimLight method
    fn claim_light(&self) -> Result<()>;

    /// ClaimProximity method
    fn claim_proximity(&self) -> Result<()>;

    /// ReleaseAccelerometer method
    fn release_accelerometer(&self) -> Result<()>;

    /// ReleaseLight method
    fn release_light(&self) -> Result<()>;

    /// ReleaseProximity method
    fn release_proximity(&self) -> Result<()>;

    /// AccelerometerOrientation property
    #[zbus(property)]
    fn accelerometer_orientation(&self) -> Result<AccelerometerOrientation>;

    /// HasAccelerometer property
    #[zbus(property)]
    fn has_accelerometer(&self) -> Result<bool>;

    /// HasAmbientLight property
    #[zbus(property)]
    fn has_ambient_light(&self) -> Result<bool>;

    /// HasProximity property
    #[zbus(property)]
    fn has_proximity(&self) -> Result<bool>;

    /// LightLevel property
    #[zbus(property)]
    fn light_level(&self) -> Result<f64>;

    /// LightLevelUnit property
    #[zbus(property)]
    fn light_level_unit(&self) -> Result<LightLevelUnit>;

    /// ProximityNear property
    #[zbus(property)]
    fn proximity_near(&self) -> Result<bool>;
}

#[derive(Debug, Clone, PartialEq, Type)]
#[zvariant(signature = "s")]
pub enum AccelerometerOrientation {
    Undefined,
    Normal,
    BottomUp,
    LeftUp,
    RightUp,

    Unknown(String),
}

impl TryFrom<OwnedValue> for AccelerometerOrientation {
    type Error = zvariant::Error;

    fn try_from(value: OwnedValue) -> std::result::Result<Self, Self::Error> {
        let Value::Str(s) = &*value else {
            return Err(zvariant::Error::IncorrectType);
        };
        let value = match s.as_str() {
            "normal" => Self::Normal,
            "bottom-up" => Self::BottomUp,
            "left-up" => Self::LeftUp,
            "right-up" => Self::RightUp,
            "undefined" => Self::Undefined,
            s => Self::Unknown(s.into()),
        };
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Type)]
#[zvariant(signature = "s")]
pub enum LightLevelUnit {
    Lux,
    Vendor,

    Unknown(String),
}

impl TryFrom<OwnedValue> for LightLevelUnit {
    type Error = zvariant::Error;

    fn try_from(value: OwnedValue) -> std::result::Result<Self, Self::Error> {
        let Value::Str(s) = &*value else {
            return Err(zvariant::Error::IncorrectType);
        };
        let value = match s.as_str() {
            "lux" => Self::Lux,
            "vendor" => Self::Vendor,
            s => Self::Unknown(s.into()),
        };
        Ok(value)
    }
}
