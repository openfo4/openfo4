use bytes::Buf;

#[derive(Clone, Debug)]
pub struct Error;

#[derive(Clone, Debug)]
pub struct AnimationSpeedInfo {}

impl AnimationSpeedInfo {
    pub fn parse(mut buf: &[u8]) -> Self {
        let version = u32::decode(&mut buf).unwrap();
        let behavior_count = u32::decode(&mut buf).unwrap();
        dbg!(&version);
        dbg!(&behavior_count);

        let hkx_name = String::decode(&mut buf).unwrap();
        dbg!(&hkx_name);

        let class_name = String::decode(&mut buf).unwrap();
        dbg!(&class_name);

        match &*class_name {
            "CollectionAnimationSpeedContourName" => {
                let count = u32::decode(&mut buf).unwrap();
                dbg!(&count);

                for _ in 0..count {}
            }
            _ => (),
        }

        todo!()
    }
}

trait Decode: Sized {
    fn decode<B>(buf: B) -> Result<Self, Error>
    where
        B: Buf;
}

macro_rules! impl_decode_primitive {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode<B>(mut buf: B) -> Result<Self, Error>
                where
                    B: Buf,
                {
                    let mut bytes = [0; size_of::<Self>()];
                    buf.copy_to_slice(&mut bytes);
                    Ok(Self::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_decode_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl Decode for String {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let len = u8::decode(&mut buf)?;
        let mut bytes = vec![0; len.into()];
        buf.copy_to_slice(&mut bytes);

        if bytes.last() == Some(&b'\0') {
            bytes.truncate(bytes.len() - 1);
        }

        Ok(Self::from_utf8(bytes).unwrap())
    }
}
