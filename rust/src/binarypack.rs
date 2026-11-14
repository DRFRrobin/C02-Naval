//! Format BinaryPack (celui de PeerJS, sérialisation « binary »).
//!
//! Proche de MessagePack mais avec ses propres codes pour les chaînes et les
//! données binaires. On l'implémente pour parler exactement le même langage
//! que la version navigateur du jeu : un joueur sur PC (Rust) peut affronter un
//! joueur sur le site web.

use serde_json::{Map, Number, Value as Json};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bin(Vec<u8>),
    Arr(Vec<Value>),
    Map(Vec<(Value, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(m) => m.iter().find(|(k, _)| matches!(k, Value::Str(s) if s == key)).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }
}

/* ---------- Encodage ---------- */

pub fn pack(v: &Value) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    pack_into(v, &mut out);
    out
}

fn pack_into(v: &Value, o: &mut Vec<u8>) {
    match v {
        Value::Nil => o.push(0xc0),
        Value::Bool(b) => o.push(if *b { 0xc3 } else { 0xc2 }),
        Value::Int(n) => pack_int(*n, o),
        Value::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 9.0e15 {
                pack_int(*f as i64, o)
            } else {
                o.push(0xcb);
                o.extend_from_slice(&f.to_be_bytes());
            }
        }
        Value::Str(s) => {
            let b = s.as_bytes();
            let n = b.len();
            if n <= 0x0f {
                o.push(0xb0 + n as u8);
            } else if n <= 0xffff {
                o.push(0xd8);
                o.extend_from_slice(&(n as u16).to_be_bytes());
            } else {
                o.push(0xd9);
                o.extend_from_slice(&(n as u32).to_be_bytes());
            }
            o.extend_from_slice(b);
        }
        Value::Bin(b) => {
            let n = b.len();
            if n <= 0x0f {
                o.push(0xa0 + n as u8);
            } else if n <= 0xffff {
                o.push(0xda);
                o.extend_from_slice(&(n as u16).to_be_bytes());
            } else {
                o.push(0xdb);
                o.extend_from_slice(&(n as u32).to_be_bytes());
            }
            o.extend_from_slice(b);
        }
        Value::Arr(a) => {
            let n = a.len();
            if n <= 0x0f {
                o.push(0x90 + n as u8);
            } else if n <= 0xffff {
                o.push(0xdc);
                o.extend_from_slice(&(n as u16).to_be_bytes());
            } else {
                o.push(0xdd);
                o.extend_from_slice(&(n as u32).to_be_bytes());
            }
            for x in a {
                pack_into(x, o);
            }
        }
        Value::Map(m) => {
            let n = m.len();
            if n <= 0x0f {
                o.push(0x80 + n as u8);
            } else if n <= 0xffff {
                o.push(0xde);
                o.extend_from_slice(&(n as u16).to_be_bytes());
            } else {
                o.push(0xdf);
                o.extend_from_slice(&(n as u32).to_be_bytes());
            }
            for (k, x) in m {
                pack_into(k, o);
                pack_into(x, o);
            }
        }
    }
}

fn pack_int(n: i64, o: &mut Vec<u8>) {
    if (-0x20..=0x7f).contains(&n) {
        o.push((n & 0xff) as u8);
    } else if (0..=0xff).contains(&n) {
        o.push(0xcc);
        o.push(n as u8);
    } else if (-0x80..=0x7f).contains(&n) {
        o.push(0xd0);
        o.push(n as i8 as u8);
    } else if (0..=0xffff).contains(&n) {
        o.push(0xcd);
        o.extend_from_slice(&(n as u16).to_be_bytes());
    } else if (-0x8000..=0x7fff).contains(&n) {
        o.push(0xd1);
        o.extend_from_slice(&(n as i16).to_be_bytes());
    } else if (0..=0xffff_ffff).contains(&n) {
        o.push(0xce);
        o.extend_from_slice(&(n as u32).to_be_bytes());
    } else if (-0x8000_0000..=0x7fff_ffff).contains(&n) {
        o.push(0xd2);
        o.extend_from_slice(&(n as i32).to_be_bytes());
    } else {
        o.push(0xd3);
        o.extend_from_slice(&n.to_be_bytes());
    }
}

/* ---------- Décodage ---------- */

struct Reader<'a> {
    d: &'a [u8],
    i: usize,
    depth: u32,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.i + n > self.d.len() {
            return None;
        }
        let s = &self.d[self.i..self.i + n];
        self.i += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|s| u16::from_be_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Option<u64> {
        self.take(8).map(|s| u64::from_be_bytes(s.try_into().unwrap()))
    }
    fn string(&mut self, n: usize) -> Option<Value> {
        self.take(n).map(|s| Value::Str(String::from_utf8_lossy(s).into_owned()))
    }
    fn raw(&mut self, n: usize) -> Option<Value> {
        self.take(n).map(|s| Value::Bin(s.to_vec()))
    }
    fn array(&mut self, n: usize) -> Option<Value> {
        if n > self.d.len() {
            return None;
        }
        let mut a = Vec::with_capacity(n);
        for _ in 0..n {
            a.push(self.value()?);
        }
        Some(Value::Arr(a))
    }
    fn map(&mut self, n: usize) -> Option<Value> {
        if n > self.d.len() {
            return None;
        }
        let mut m = Vec::with_capacity(n);
        for _ in 0..n {
            let k = self.value()?;
            let v = self.value()?;
            m.push((k, v));
        }
        Some(Value::Map(m))
    }
    fn value(&mut self) -> Option<Value> {
        self.depth += 1;
        if self.depth > 64 {
            return None;
        }
        let r = self.value_inner();
        self.depth -= 1;
        r
    }
    fn value_inner(&mut self) -> Option<Value> {
        let t = self.u8()?;
        if t < 0x80 {
            return Some(Value::Int(t as i64));
        }
        if (t ^ 0xe0) < 0x20 {
            return Some(Value::Int((t ^ 0xe0) as i64 - 0x20));
        }
        if (t ^ 0xa0) <= 0x0f {
            return self.raw((t ^ 0xa0) as usize);
        }
        if (t ^ 0xb0) <= 0x0f {
            return self.string((t ^ 0xb0) as usize);
        }
        if (t ^ 0x90) <= 0x0f {
            return self.array((t ^ 0x90) as usize);
        }
        if (t ^ 0x80) <= 0x0f {
            return self.map((t ^ 0x80) as usize);
        }
        match t {
            0xc0 | 0xc1 | 0xd4..=0xd7 => Some(Value::Nil),
            0xc2 => Some(Value::Bool(false)),
            0xc3 => Some(Value::Bool(true)),
            0xca => self.u32().map(|b| Value::Float(f32::from_bits(b) as f64)),
            0xcb => self.u64().map(|b| Value::Float(f64::from_bits(b))),
            0xcc => self.u8().map(|v| Value::Int(v as i64)),
            0xcd => self.u16().map(|v| Value::Int(v as i64)),
            0xce => self.u32().map(|v| Value::Int(v as i64)),
            0xcf => self.u64().map(|v| Value::Int(v as i64)),
            0xd0 => self.u8().map(|v| Value::Int(v as i8 as i64)),
            0xd1 => self.u16().map(|v| Value::Int(v as i16 as i64)),
            0xd2 => self.u32().map(|v| Value::Int(v as i32 as i64)),
            0xd3 => self.u64().map(|v| Value::Int(v as i64)),
            0xd8 => {
                let n = self.u16()? as usize;
                self.string(n)
            }
            0xd9 => {
                let n = self.u32()? as usize;
                self.string(n)
            }
            0xda => {
                let n = self.u16()? as usize;
                self.raw(n)
            }
            0xdb => {
                let n = self.u32()? as usize;
                self.raw(n)
            }
            0xdc => {
                let n = self.u16()? as usize;
                self.array(n)
            }
            0xdd => {
                let n = self.u32()? as usize;
                self.array(n)
            }
            0xde => {
                let n = self.u16()? as usize;
                self.map(n)
            }
            0xdf => {
                let n = self.u32()? as usize;
                self.map(n)
            }
            _ => None,
        }
    }
}

pub fn unpack(d: &[u8]) -> Option<Value> {
    Reader { d, i: 0, depth: 0 }.value()
}

/* ---------- Conversion JSON <-> BinaryPack ---------- */

pub fn from_json(j: &Json) -> Value {
    match j {
        Json::Null => Value::Nil,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        Json::String(s) => Value::Str(s.clone()),
        Json::Array(a) => Value::Arr(a.iter().map(from_json).collect()),
        Json::Object(m) => Value::Map(m.iter().map(|(k, v)| (Value::Str(k.clone()), from_json(v))).collect()),
    }
}

pub fn to_json(v: &Value) -> Json {
    match v {
        Value::Nil => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(i) => Json::Number((*i).into()),
        Value::Float(f) => Number::from_f64(*f).map(Json::Number).unwrap_or(Json::Null),
        Value::Str(s) => Json::String(s.clone()),
        Value::Bin(_) => Json::Null,
        Value::Arr(a) => Json::Array(a.iter().map(to_json).collect()),
        Value::Map(m) => {
            let mut o = Map::new();
            for (k, x) in m {
                let key = match k {
                    Value::Str(s) => s.clone(),
                    Value::Int(i) => i.to_string(),
                    _ => continue,
                };
                o.insert(key, to_json(x));
            }
            Json::Object(o)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roundtrip() {
        let j = json!({"t":"res","kind":"cross","at":[3,4],"results":[{"r":3,"c":4,"v":"sunk","ship":[[3,4],[3,5]],"si":4}],"over":false,"big":70000,"neg":-300,"name":"Invité é"});
        let b = pack(&from_json(&j));
        assert_eq!(to_json(&unpack(&b).unwrap()), j);
    }

    #[test]
    fn long_string() {
        let s = "x".repeat(70000);
        let b = pack(&Value::Str(s.clone()));
        assert_eq!(b[0], 0xd9);
        assert_eq!(unpack(&b), Some(Value::Str(s)));
    }
}
