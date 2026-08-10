#![allow(dead_code, unused)]
use std::{collections::HashMap, fs};

#[derive(Default)]
pub struct Obj {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn load_obj_from_string(file: &str) -> Obj {
    let mut p = Vec::new();
    let mut uv = Vec::new();
    let mut out = Obj::default();
    let mut map = HashMap::new();

    for l in file.lines() {
        let mut x = l.split_whitespace();
        match x.next() {
            Some("v") => p.push([
                x.next().unwrap().parse().unwrap(),
                x.next().unwrap().parse().unwrap(),
                x.next().unwrap().parse().unwrap(),
            ]),
            Some("vt") => uv.push([x.next().unwrap().parse().unwrap(), x.next().unwrap().parse().unwrap()]),
            Some("f") => {
                let mut face = Vec::new();

                for v in x {
                    // v/vt or v/vt/vn or v//vn
                    let mut i = v.split('/');
                    let pi: usize = i.next().unwrap().parse().unwrap();
                    let ti: usize = i.next().unwrap().parse().unwrap();

                    let key = (pi, ti);
                    let n = *map.entry(key).or_insert_with(|| {
                        let n = out.positions.len() as u32;
                        out.positions.push(p[pi - 1]);
                        out.uvs.push(uv[ti - 1]);
                        n
                    });

                    face.push(n);
                }

                // fan-triangulate polygon
                for i in 2..face.len() {
                    out.indices.extend([face[0], face[i - 1], face[i]]);
                }
            }
            _ => {}
        }
    }

    out
}

pub fn load_obj(path: &std::path::Path) -> Obj {
    let s = fs::read_to_string(path).unwrap();

    load_obj_from_string(&s)
}
