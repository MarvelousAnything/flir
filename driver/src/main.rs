use std::{error::Error, time::Duration};

use rusb::{
    open_device_with_vid_pid, set_log_level, DeviceHandle, EndpointDescriptor, GlobalContext,
};

#[derive(Debug)]
pub enum ProtocolType {
    CONFIG,
    FILEIO,
    FRAME,
}

#[derive(Debug)]
pub struct FlirOne<'a> {
    handle: DeviceHandle<GlobalContext>,
    config: (EndpointDescriptor<'a>, EndpointDescriptor<'a>),
    frame: (EndpointDescriptor<'a>, EndpointDescriptor<'a>),
    fileio: (EndpointDescriptor<'a>, EndpointDescriptor<'a>),
    connected: bool,
    expect_file_data: bool,
    expect_frame_data: bool,
}

impl<'a> FlirOne<'a> {
    pub fn toggle_communication(
        &mut self,
        protocol_type: ProtocolType,
        start: bool,
    ) -> Result<(), Box<dyn Error>> {
        let control_cmd = if start { 1 } else { 0 };
        let index = match protocol_type {
            ProtocolType::CONFIG => 0,
            ProtocolType::FILEIO => {
                self.expect_file_data = true;
                1
            }
            ProtocolType::FRAME => {
                self.expect_frame_data = true;
                2
            }
        };

        let res = self.handle.write_control(
            0x1,
            11,
            control_cmd,
            index,
            &Vec::new(),
            Duration::from_secs(1),
        )?;
        println!("res {res}");
        Ok(())
    }

    pub fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.connected {
            self.connected = true;
            self.toggle_communication(ProtocolType::FILEIO, true)?;
        }
        Ok(())
    }
}

pub struct FlirOneBuilder<'a> {
    config_read: Option<EndpointDescriptor<'a>>,
    config_write: Option<EndpointDescriptor<'a>>,
    frame_read: Option<EndpointDescriptor<'a>>,
    frame_write: Option<EndpointDescriptor<'a>>,
    fileio_read: Option<EndpointDescriptor<'a>>,
    fileio_write: Option<EndpointDescriptor<'a>>,

    handle: DeviceHandle<GlobalContext>,
}

impl<'a> FlirOneBuilder<'a> {
    pub fn new(handle: DeviceHandle<GlobalContext>) -> Self {
        FlirOneBuilder {
            config_read: None,
            config_write: None,
            frame_read: None,
            frame_write: None,
            fileio_read: None,
            fileio_write: None,
            handle,
        }
    }

    pub fn config_read(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.config_read = Some(endpoint);
        self
    }

    pub fn config_write(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.config_write = Some(endpoint);
        self
    }

    pub fn frame_read(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.frame_read = Some(endpoint);
        self
    }

    pub fn frame_write(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.frame_write = Some(endpoint);
        self
    }

    pub fn fileio_read(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.fileio_read = Some(endpoint);
        self
    }

    pub fn fileio_write(mut self, endpoint: EndpointDescriptor<'a>) -> Self {
        self.fileio_write = Some(endpoint);
        self
    }

    pub fn build(self) -> Result<FlirOne<'a>, &'static str> {
        Ok(FlirOne {
            handle: self.handle,
            config: (
                self.config_read.ok_or("config_read not set")?,
                self.config_write.ok_or("config_write not set")?,
            ),
            frame: (
                self.frame_read.ok_or("frame_read not set")?,
                self.frame_write.ok_or("frame_write not set")?,
            ),
            fileio: (
                self.fileio_read.ok_or("fileio_read not set")?,
                self.fileio_write.ok_or("fileio_write not set")?,
            ),
            connected: false,
            expect_file_data: false,
            expect_frame_data: false,
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    set_log_level(rusb::LogLevel::Debug);

    let flir = open_device_with_vid_pid(0x09CB, 0x1996).unwrap();
    println!("{flir:#?}");

    let mut builder = FlirOneBuilder::new(flir);

    let config = builder.handle.device().active_config_descriptor()?;
    for interface in config.interfaces() {
        builder.handle.claim_interface(interface.number())?;
        for descriptor in interface.descriptors() {
            for endpoint in descriptor.endpoint_descriptors() {
                // println!(
                //     "{:?} - {:?} - {:?} - {:?}",
                //     endpoint.direction(),
                //     endpoint.transfer_type(),
                //     endpoint.sync_type(),
                //     endpoint.usage_type()
                // );
                builder = match endpoint.direction() {
                    rusb::Direction::In => match endpoint.number() {
                        1 => builder.config_read(endpoint),
                        3 => builder.fileio_read(endpoint),
                        5 => builder.frame_read(endpoint),
                        n => panic!("invalid endpoint # {n}"),
                    },
                    rusb::Direction::Out => match endpoint.number() {
                        2 => builder.config_write(endpoint),
                        4 => builder.fileio_write(endpoint),
                        6 => builder.frame_write(endpoint),
                        n => panic!("invalid endpoint # {n}"),
                    },
                };
            }
        }
    }

    let mut flir = builder.build()?;
    let mut buf = [0u8; 4096];
    flir.connect()?;
    flir.toggle_communication(ProtocolType::FRAME, true)?;
    println!("{flir:#?}");

    println!("address {}", flir.frame.0.address());
    flir.handle
        .read_bulk(flir.config.0.address(), &mut buf, Duration::from_secs(30))?;
    println!("{buf:?}");
    let mut frame_buf = [0u8; 131072];
    flir.handle
        .read_bulk(flir.frame.0.address(), &mut frame_buf, Duration::from_secs(30))?;
    println!("{frame_buf:?}");
    Ok(())
}
