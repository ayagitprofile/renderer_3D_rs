#![allow(dead_code, unused)]

use std::{collections::HashMap, fs};

#[derive(Default)]
pub struct Obj {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub indices: Vec<u32>,
}

pub fn load_obj_from_string(file: &str) -> Obj {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut has_uvs = false;
    let mut out = Obj::default();
    let mut map = HashMap::new();
    for line in file.lines() {
        let mut x = line.split_whitespace();
        match x.next() {
            Some("v") => {
                positions.push([
                    x.next().unwrap().parse().unwrap(),
                    x.next().unwrap().parse().unwrap(),
                    x.next().unwrap().parse().unwrap(),
                ]);
            }
            Some("vt") => {
                has_uvs = true;
                uvs.push([x.next().unwrap().parse().unwrap(), x.next().unwrap().parse().unwrap()]);
            }
            Some("f") => {
                let mut face = Vec::new();
                for v in x {
                    let mut indices = v.split('/');
                    let pi: usize = indices.next().unwrap().parse().unwrap();
                    let ti = indices
                        .next()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse::<usize>().unwrap());
                    let key = (pi, ti);
                    let n = *map.entry(key).or_insert_with(|| {
                        let n = out.positions.len() as u32;
                        out.positions.push(positions[pi - 1]);
                        if let Some(ti) = ti {
                            if out.uvs.is_none() {
                                out.uvs = Some(Vec::new());
                            }
                            out.uvs.as_mut().unwrap().push(uvs[ti - 1]);
                        } else if let Some(out_uvs) = &mut out.uvs {
                            out_uvs.push([0.0, 0.0]);
                        }
                        n
                    });
                    face.push(n);
                }
                for i in 2..face.len() {
                    out.indices.extend([face[0], face[i - 1], face[i]]);
                }
            }
            _ => {}
        }
    }
    if has_uvs {
        out.uvs = Some(uvs);
    }
    out
}

pub fn load_obj(path: &std::path::Path) -> Obj {
    let s = fs::read_to_string(path).unwrap();
    load_obj_from_string(&s)
}
