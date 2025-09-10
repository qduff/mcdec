use std::io::{self, Error, ErrorKind, Read};

/// todo this really doesnt need to borrow
pub struct BinaryReader<'a, R>
where
    R: Read,
{
    reader: &'a mut R,
    local_position: u64,
    remaining: u64,
}

impl<'a, R: Read> BinaryReader<'a, R> {
    pub fn new(storage: &'a mut R, len: u64) -> BinaryReader<'a, R> {
        BinaryReader {
            local_position: 0,
            reader: storage,
            remaining: len,
        }
    }

    pub fn mut_slice<'b>(&'b mut self, len: u64) -> BinaryReader<'b, R> {
        if len > self.remaining {
            panic!("Attempting a larger mut_slice than available!")
        }
        self.remaining -= len;
        self.local_position += len;
        BinaryReader {
            local_position: 0,
            reader: self.reader,
            remaining: len,
        }
    }
    pub fn get_local_position(&self) -> u64 {
        self.local_position
    }

    pub fn get_remaining(&self) -> u64 {
        self.remaining
    }

    pub fn has_remaining(&mut self) -> bool {
        self.remaining > 0
    }

    pub fn has_enough(&mut self, count: u64) -> io::Result<()> {
        if count > self.remaining {
            Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!(
                    "Attempted to read {} bytes with only {} remaining in reader",
                    count, self.remaining
                ),
            ))
        } else {
            Ok(())
        }
    }

    pub fn read_n_bytes(&mut self, n: u64) -> io::Result<Vec<u8>> {
        self.has_enough(n)?;
        self.remaining -= n;
        self.local_position += n;
        let mut v = vec![0u8; n as usize];
        self.reader.read_exact(&mut v)?;
        Ok(v)
    }

    pub fn new_string(&mut self, len: u64) -> io::Result<String> {
        String::from_utf8(self.read_n_bytes(len)?).map_err(|_| Error::other("Invalid UTF8"))
    }

    #[inline]
    pub fn read_bytes<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut buffer = [0u8; N];
        self.has_enough(N as u64)?;
        self.remaining -= N as u64;
        self.local_position += N as u64;
        self.reader.read_exact(&mut buffer)?;
        // println!("{:x?}", buffer);
        Ok(buffer)
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        Ok(u8::from_be_bytes(self.read_bytes::<1>()?))
    }

    pub fn read_i8(&mut self) -> io::Result<i8> {
        Ok(i8::from_be_bytes(self.read_bytes::<1>()?))
    }

    pub fn read_u16(&mut self) -> io::Result<u16> {
        const NUM_BYTES: usize = std::mem::size_of::<u16>();
        Ok(u16::from_be_bytes(self.read_bytes::<NUM_BYTES>()?))
    }

    pub fn read_i16(&mut self) -> io::Result<i16> {
        const NUM_BYTES: usize = std::mem::size_of::<i16>();
        Ok(i16::from_be_bytes(self.read_bytes::<NUM_BYTES>()?))
    }

    // pub fn read<T: >(&mut self) -> io::Result<T>{
    //     const NUM_BYTES: usize = mem::size_of::<T>();
    //     Ok(T::from_be_bytes(self.read_bytes::<NUM_BYTES>()?))
    // }
    pub fn read_u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.read_bytes::<4>()?))
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_be_bytes(self.read_bytes::<4>()?))
    }

    pub fn read_f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_be_bytes(self.read_bytes::<4>()?))
    }

    pub fn read_i64(&mut self) -> io::Result<i64> {
        Ok(i64::from_be_bytes(self.read_bytes::<8>()?))
    }

    pub fn read_u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(self.read_bytes::<8>()?))
    }

    pub fn read_f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_be_bytes(self.read_bytes::<8>()?))
    }

    pub fn consume(&mut self, amount: u64) -> io::Result<()> {
        if self.remaining == 0 {
            return Ok(());
        }
        self.has_enough(amount)?;
        self.remaining -= amount;
        self.local_position += amount;
        io::copy(&mut self.reader.take(amount), &mut io::sink())?;
        Ok(())
    }
}

impl<R: Read> Drop for BinaryReader<'_, R> {
    fn drop(&mut self) {
        self.consume(self.remaining).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sequence {
        current: u8,
        limit: u8,
    }

    impl Sequence {
        fn new(limit: u8) -> Sequence {
            Sequence { current: 1, limit }
        }

        fn get_next(&mut self) -> u8 {
            if self.current <= self.limit {
                let result = self.current;
                self.current += 1;
                result
            } else {
                panic!("Sequence exhausted!");
            }
        }
    }

    impl Read for Sequence {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            buf[0] = self.get_next();
            Ok(1)
        }
    }

    #[test]
    fn valid_nesting() {
        let size: u8 = 10;

        let mut seq: Sequence = Sequence::new(size);
        let mut binreader = BinaryReader::new(&mut seq, size as u64);

        assert_eq!(binreader.read_u8().unwrap(), 1);
        assert_eq!(binreader.read_u8().unwrap(), 2);

        {
            let mut view1 = binreader.mut_slice(6);
            {
                let mut view11 = view1.mut_slice(2);
                assert_eq!(view11.read_u8().unwrap(), 3);
            }
            {
                let mut view11 = view1.mut_slice(3);
                assert_eq!(view11.read_u8().unwrap(), 5);
            }
            assert_eq!(view1.read_u8().unwrap(), 8);
        }
        assert_eq!(binreader.read_u8().unwrap(), 9);
    }

    #[test]
    fn valid_slice() {
        let size: u8 = 10;
        let mut seq: Sequence = Sequence::new(size);
        let mut binreader = BinaryReader::new(&mut seq, size as u64);
        binreader.mut_slice(size as u64);
    }

    #[test]
    #[should_panic]
    fn invalid_slice() {
        let size: u8 = 10;
        let mut seq: Sequence = Sequence::new(size);
        let mut binreader = BinaryReader::new(&mut seq, size as u64);
        binreader.mut_slice(size as u64 + 1);
    }

    #[test]
    #[should_panic]
    fn read_too_many() {
        let size: u8 = 10;
        let mut seq: Sequence = Sequence::new(size);
        let mut binreader = BinaryReader::new(&mut seq, size as u64);
        (1..=size + 1).for_each(|_| drop(binreader.read_u8().unwrap()));
    }
}
