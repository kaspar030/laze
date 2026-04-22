use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};

pub trait Format<T: Sized> {
    fn from_buffer(reader: &mut BufReader<File>) -> Result<T, Error>;
    fn to_buffer(writer: &mut BufWriter<File>, value: &T) -> Result<(), Error>;
}

pub struct Bincode {}

impl<T> Format<T> for Bincode
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn from_buffer(reader: &mut BufReader<File>) -> Result<T, Error> {
        let res: T = bincode::deserialize_from(reader)?;

        Ok(res)
    }

    fn to_buffer(writer: &mut BufWriter<File>, value: &T) -> Result<(), Error> {
        bincode::serialize_into(writer, value)?;

        Ok(())
    }
}

impl<T> Cached<T> for T where T: Serialize + for<'de> Deserialize<'de> {}

pub trait Cached<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn load_from_cache<F: Format<T>>(file: impl AsRef<Path>) -> Result<T, Error> {
        let file = File::open(file.as_ref())?;

        let mut buffer = BufReader::new(file);

        let build_uuid: [u8; 16] = bincode::deserialize_from(&mut buffer)?;
        if &build_uuid != build_uuid::get().as_bytes() {
            return Err(anyhow!("cache from different laze version"));
        }

        let res = F::from_buffer(&mut buffer)?;

        Ok(res)
    }

    fn to_cache<F: Format<T>>(value: &T, file: impl AsRef<Path>) -> Result<(), Error> {
        let file = File::create(file.as_ref())?;
        let mut buffer = std::io::BufWriter::new(file);

        bincode::serialize_into(&mut buffer, &build_uuid::get().as_bytes())?;

        F::to_buffer(&mut buffer, value)?;
        Ok(())
    }
}
