use std::collections::VecDeque;

use bytes::Buf;

use crate::{Decode, Error, Field, FieldHeader, GenericField, Text};

#[derive(Clone, Debug)]
pub(crate) struct FieldMap {
    fields: VecDeque<GenericField>,
}

impl FieldMap {
    /// `end` is exclusive.
    pub(crate) fn split_off(&mut self, end: Text<4>) -> Self {
        let index = self.fields.iter().position(|f| f.key == end).unwrap_or(0);
        let fields = self.fields.drain(0..index).collect();
        FieldMap { fields }
    }

    pub(crate) fn next_key(&self) -> Option<Text<4>> {
        self.fields.front().map(|f| f.key)
    }

    /// Consumes the next field if it has the given key. Returns `None` if the next field does not
    /// match the given key.
    pub(crate) fn consume<T>(&mut self, key: Text<4>) -> Result<Option<Field<T>>, T::Error>
    where
        T: Decode,
    {
        let Some(index) = self.fields.iter().position(|f| f.key == key) else {
            return Ok(None);
        };

        let field = self.fields.remove(index).unwrap();
        field.to_field().map(Some)
    }

    pub(crate) fn consume_next<T>(&mut self, key: Text<4>) -> Result<Option<Field<T>>, T::Error>
    where
        T: Decode,
    {
        // This is somewhat like Vec::pop_if, but for VecDeque.
        if self.fields.front().is_none_or(|f| f.key != key) {
            return Ok(None);
        }
        let field = self.fields.pop_front().unwrap();

        field.to_field().map(Some)
    }

    fn count(&self, key: Text<4>) -> usize {
        self.fields.iter().filter(|field| field.key == key).count()
    }
}

impl Decode for FieldMap {
    type Error = Error;

    fn decode<B>(mut buf: B) -> Result<Self, Self::Error>
    where
        B: Buf,
    {
        let mut fields = VecDeque::new();
        let mut predecing_xxxx_value = None;
        while buf.has_remaining() {
            let mut header = FieldHeader::decode(&mut buf)?;

            // If the preceding field has key XXXX, size on the current
            // field will be 0 and the size is instead stored inside
            // the XXXX field.
            if &header.key.0 == b"XXXX" {
                let field = header.read_value(&mut buf)?;
                let xxxx_field = field.to_field::<u32>().unwrap();
                predecing_xxxx_value = Some(xxxx_field.value);

                continue;
            }

            if header.size == 0 {
                if let Some(size) = predecing_xxxx_value.take() {
                    header.size = size;
                }
            }

            let field = header.read_value(&mut buf)?;

            fields.push_back(field);
        }

        Ok(Self { fields })
    }
}

pub(crate) trait Read: Sized {
    type Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error>;

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error>;
}

impl<T> Read for T
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Option::<T>::read(map, key)?.ok_or(Error::MissingFieldText(key))
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        Option::<T>::read_next(map, key)?.ok_or(Error::MissingFieldText(key))
    }
}

impl<T> Read for Option<T>
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = T::Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        map.consume(key).map(|f| f.map(|f| f.value))
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        map.consume_next(key).map(|f| f.map(|f| f.value))
    }
}

impl<T> Read for Vec<T>
where
    T: Decode,
    Error: From<T::Error>,
{
    type Error = T::Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        while let Some(field) = map.consume(key)? {
            fields.push(field.value);
        }

        Ok(fields)
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        while let Some(field) = map.consume_next(key)? {
            fields.push(field.value);
        }

        Ok(fields)
    }
}

/// An array of fields encoded as separate fields with the same key.
#[derive(Clone, Debug)]
pub struct SubRecordArray<T>(pub Vec<T>);

impl<T> Read for SubRecordArray<T>
where
    T: Read,
    Error: From<T::Error>,
{
    type Error = T::Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        let mut elems = Vec::new();
        while map.next_key().is_some_and(|k| k == key) {
            let elem = T::read(map, key)?;
            elems.push(elem);
        }

        Ok(Self(elems))
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        let mut elems = Vec::new();
        while map.next_key().is_some_and(|k| k == key) {
            let elem = T::read(map, key)?;
            elems.push(elem);
        }

        Ok(Self(elems))
    }
}

/// A partial subrecord that is entered with a marker field.
///
/// For example there are commonly `MNAM` and `FNAM` fields for male and female respectively, which
/// then both contain the same fields.
///
/// This type can be used to detect a marker field and continue parsing at the correct location or
/// skip the section.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubRecordSection<T>(pub T);

impl<T> Read for Option<SubRecordSection<T>>
where
    T: Read<Error = Error>,
{
    type Error = T::Error;

    fn read(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        match map.consume::<()>(key)? {
            Some(_) => {
                let section = T::read(map, key)?;
                Ok(Some(SubRecordSection(section)))
            }
            None => Ok(None),
        }
    }

    fn read_next(map: &mut FieldMap, key: Text<4>) -> Result<Self, Self::Error> {
        match map.consume_next::<()>(key)? {
            Some(_) => {
                let section = T::read(map, key)?;
                Ok(Some(SubRecordSection(section)))
            }
            None => Ok(None),
        }
    }
}
