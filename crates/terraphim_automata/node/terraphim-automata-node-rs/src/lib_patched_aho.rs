#[macro_use]
extern crate neon;
extern crate aho_corasick;

use neon::prelude::*;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind, PatternID};


declare_types! {
    pub class JsAhoCorasick for AhoCorasick {
        init(mut cx) {
            let arr = cx.argument::<JsArray>(0)?;
            let inputs = arr.to_vec(&mut cx)?;

            let patterns: Vec<String> = inputs.into_iter().map(|x| x.downcast::<JsString>().unwrap().value()).collect();
            let ac =  AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(patterns.clone())
            .unwrap();
            Ok(ac)
        }

        method find_iter(mut cx) {
            let this = cx.this();
            let s = cx.argument::<JsString>(0)?;

            let mut matches = vec![];

            {
                let guard = cx.lock();
                let ac = this.borrow(&guard);

                for mat in ac.find_iter(&s.value()) {
                    matches.push(vec![mat.pattern().as_usize(), mat.start(), mat.end()]);
                }
            }

            let js_array = JsArray::new(&mut cx, matches.len() as u32);

            for (i, obj) in matches.iter().enumerate() {
                let inner_arr = JsArray::new(&mut cx, 3);
                for j in 0..3 {
                    let js_num = cx.number(obj[j] as f64);
                    inner_arr.set(&mut cx, j as u32, js_num).unwrap();
                }

                js_array.set(&mut cx, i as u32, inner_arr).unwrap();
            }

            Ok(js_array.upcast())
        }
     }
    }


    register_module!( mut cx, {
        cx.export_class::<JsAhoCorasick>("AhoCorasick")
    });
