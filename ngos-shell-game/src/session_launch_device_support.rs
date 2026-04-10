use alloc::string::String;

pub struct SessionDeviceDefaults {
    pub graphics_device_path: String,
    pub graphics_driver_path: String,
    pub audio_device_path: String,
    pub audio_driver_path: String,
    pub input_device_path: String,
    pub input_driver_path: String,
}

pub fn default_session_device_paths() -> SessionDeviceDefaults {
    SessionDeviceDefaults {
        graphics_device_path: String::from("/dev/gpu0"),
        graphics_driver_path: String::from("/drv/gpu0"),
        audio_device_path: String::from("/dev/audio0"),
        audio_driver_path: String::from("/drv/audio0"),
        input_device_path: String::from("/dev/input0"),
        input_driver_path: String::from("/drv/input0"),
    }
}
