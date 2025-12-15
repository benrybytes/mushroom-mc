static SEGMENT_BITS: i32 = 0x7F;
static CONTINUE_BIT: i32 = 0x80;

use crate::globals::VARNUM_ERROR;

use super::PacketHandler;
use log::*;
use tokio::io;

impl<'a> PacketHandler<'a> {
    pub fn size_varint(mut value: i32) -> u32 {
        let mut size: u32 = 1;
        while (value & !SEGMENT_BITS) != 0 {
            value >>= 7;
            size += 1;
        }
        return size;
    }
}

impl<'a> PacketHandler<'a> {
    pub async fn read_varint(&mut self) -> Result<i32, i32> {
        let mut value: i32 = 0;
        let mut position: i32 = 0;
        loop {
            if let Ok(current_byte) = self.read_u8().await {
                value |= ((current_byte & SEGMENT_BITS as u8) as i32) << position;
                if (current_byte & CONTINUE_BIT as u8) == 0 {
                    break;
                }

                position += 7;

                if position >= 32 {
                    error!("ERROR MOVINGN POSITION BY 32");
                    return Err(VARNUM_ERROR);
                }
            } else {
                warn! {"could not read byte"};
                break;
            }
        }
        debug!(
            "
        successfully made varnum
        {value}

        "
        );
        return Ok(value);
    }

    /// hello

    pub async fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
        loop {
            if (value & !SEGMENT_BITS) == 0 {
                self.write_byte(value as u8).await?;
                break;
            }

            self.write_byte(((value & SEGMENT_BITS) | CONTINUE_BIT) as u8)
                .await?;

            value >>= 7;
        }
        Ok(())
    }
}
