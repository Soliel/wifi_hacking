use defmt::{Format, info};
use embassy_rp::i2c::{Async, I2c, Instance};

pub struct TLV493D<'d, T: Instance> {
    i2c_addr: u16,
    read_buffer: [u8; 10],
    write_buffer: [u8; 4],
    last_frm: u8,
    pub i2c_dev: I2c<'d, T, Async>
}

#[derive(Format)]
pub struct MagnetVals {
    x: f32,
    y: f32,
    z: f32
}

pub struct ReadMask {
    byte_num: usize,
    byte_mask: u8,
    byte_shift: u8
}

impl ReadMask {
    const fn from(vals: (i32, i32, i32)) -> Self {
        Self {
            byte_num: vals.0 as usize,
            byte_mask: vals.1 as u8,
            byte_shift: vals.2 as u8
        }
    }
}

pub struct WriteMask {
    byte_num: usize,
    byte_mask: u8,
    byte_shift: u8
}

impl WriteMask {
    const fn from(vals: (i32, i32, i32)) -> Self {
        Self {
            byte_num: vals.0 as usize,
            byte_mask: vals.1 as u8,
            byte_shift: vals.2 as u8
        }
    }
}

impl const From<(i32, i32, i32)> for ReadMask {
    fn from(value: (i32, i32, i32)) -> Self {
        ReadMask {
            byte_num: value.0 as usize,
            byte_mask: value.1 as u8,
            byte_shift: value.2 as u8
        }
    }
}

impl const From<(i32, i32, i32)> for WriteMask {
    fn from(value: (i32, i32, i32)) -> Self {
        WriteMask {
            byte_num: value.0 as usize,
            byte_mask: value.1 as u8,
            byte_shift: value.2 as u8
        }
    }
}

// Read Masks
pub const BX0:   ReadMask = ReadMask::from((0, 0xFF, 0));
pub const BX1:   ReadMask = ReadMask::from((4, 0xF0, 4));
pub const BY0:   ReadMask = ReadMask::from((1, 0xFF, 0));
pub const BY1:   ReadMask = ReadMask::from((4, 0x0F, 0));
pub const BZ0:   ReadMask = ReadMask::from((2, 0xFF, 0));
pub const BZ1:   ReadMask = ReadMask::from((5, 0x0F, 0));
pub const TEMP:  ReadMask = ReadMask::from((3, 0xF0, 4));
pub const TEMP2: ReadMask = ReadMask::from((6, 0xFF, 0));
pub const FRM:   ReadMask = ReadMask::from((3, 0x0C, 2));
pub const CH:    ReadMask = ReadMask::from((3, 0x03, 0));
pub const POWERDOWNFLAG: ReadMask = ReadMask::from((5, 0x10, 4));
pub const READRES1: ReadMask = ReadMask::from((7, 0x18, 3));
pub const READRES2: ReadMask = ReadMask::from((8, 0xFF, 0));
pub const READRES3: ReadMask = ReadMask::from((9, 0x1F, 0));

// Write Masks
pub const PARITY: WriteMask = WriteMask::from((1, 0x80, 7));
pub const ADDR: WriteMask = WriteMask::from((1, 0x60, 5));
pub const INT: WriteMask = WriteMask::from((1, 0x04, 2));
pub const FAST: WriteMask = WriteMask::from((1, 0x02, 1));
pub const LOWPOWER: WriteMask = WriteMask::from((1, 0x01, 0));
pub const TEMP_DISABLE: WriteMask = WriteMask::from((3, 0x80, 7));
pub const LP_PERIOD: WriteMask = WriteMask::from((3, 0x40, 6));
pub const POWERDOWN: WriteMask = WriteMask::from((3, 0x20, 5));
pub const WRITERES1: WriteMask = WriteMask::from((1, 0x18, 3));
pub const WRITERES2: WriteMask = WriteMask::from((2, 0xFF, 0));
pub const WRITERES3: WriteMask = WriteMask::from((3, 0x1F, 0));

impl<'d, T: Instance> TLV493D<'d, T> {
    pub fn new(i2c_dev: I2c<'d, T, Async>, addr: u16) -> Self {
        TLV493D {
            i2c_addr: addr,
            last_frm: 0,
            read_buffer: [0; 10],
            write_buffer: [0; 4],
            i2c_dev,
        }
    }

    pub async fn init(&mut self) {
        self.setup_write_buffer().await;
        self.set_write_data(ADDR, 0);
        self.set_write_data(PARITY, 1);
        self.set_write_data(FAST, 1);
        self.set_write_data(LOWPOWER, 1);
        self.write_device().await;
    }

    /// set_write_data will set a specific value in the write buffer. It gets rid of all the bit bashing.
    pub fn set_write_data(&mut self, msg_type: WriteMask, value: u8) {
        let mut current_byte = self.write_buffer[msg_type.byte_num];
        current_byte &= !msg_type.byte_mask;
        current_byte |= value << msg_type.byte_shift;
        self.write_buffer[msg_type.byte_num] = current_byte;
    }

    /// get_read_data will isolate a specific value from the Read buffer and pull it out.
    pub fn get_read_data(&mut self, msg_type: ReadMask) -> u8 {
        let raw_read_val = self.read_buffer[msg_type.byte_num];
        (raw_read_val & msg_type.byte_mask) >> msg_type.byte_shift
    }

    pub async fn read_device(&mut self) {
        self.i2c_dev.read_async(self.i2c_addr, &mut self.read_buffer).await.unwrap();
        info!("Read data: {:X}", self.read_buffer);
    }

    pub async fn write_device(&mut self) {
        info!("Write Data: {:X}", self.write_buffer);
        self.i2c_dev.write_async(self.i2c_addr, self.write_buffer).await.unwrap();
    }

    async fn setup_write_buffer(&mut self) {
        self.read_device().await;
        let res1 = self.get_read_data(READRES1);
        let res2 = self.get_read_data(READRES2);
        let res3 = self.get_read_data(READRES3);

        self.set_write_data(WRITERES1, res1);
        self.set_write_data(WRITERES2, res2);
        self.set_write_data(WRITERES3, res3);
    }

    pub async fn get_sensor_reading(&mut self) -> MagnetVals {
        self.read_device().await;
        let x_top = self.get_read_data(BX0);
        let y_top = self.get_read_data(BY0);
        let z_top = self.get_read_data(BZ0);
        let x_bot = self.get_read_data(BX1);
        let y_bot = self.get_read_data(BY1);
        let z_bot = self.get_read_data(BZ1);
        info!("raw bytes: \n\tX-TOP: {:X}, X-BOT: {:X}\n\tY-TOP: {:X}, Y-BOT: {:X}\n\tZ-TOP: {:X}, Z-BOT: {:X}",
            x_top, x_bot, y_top, y_bot, z_top, z_bot);

        let x = (self.get_read_data(BX0) as i8 as i16) << 4 | self.get_read_data(BX1) as i16;
        let y = (self.get_read_data(BY0) as i8 as i16) << 4 | self.get_read_data(BY1) as i16;
        let z = (self.get_read_data(BZ0) as i8 as i16) << 4 | self.get_read_data(BZ1) as i16;
        info!("vals no convert: X: {}, Y: {}, Z: {}", x, y, z);

        MagnetVals {
            x: x as f32 * 0.098f32,
            y: y as f32 * 0.098f32,
            z: z as f32 * 0.098f32,
        }
    }
}
