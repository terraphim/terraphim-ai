#[macro_use]
extern crate neon;
extern crate aho_corasick;

use neon::prelude::*;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind, PatternID};


fn find_matches<'a, C: Context<'a>>(patterns: Vec<&str>, haystack: &str,return_positions: bool, mut cx: FunctionContext) -> JsResult<'a,JsArray> {
    let ac = AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(patterns.clone())
        .unwrap();

    let a = cx.empty_array();
    let mut i=0;
    for mat in ac.find_iter(&haystack) {
        let term = &patterns[mat.pattern()];
        let term = cx.string(term);
        let pos = if return_positions { Some((mat.start(), mat.end())) } else { None };
        a.set(&mut cx, i as u32, term).unwrap();
        i+=1;
    }
    Ok(a)
}


fn find_matches_node(mut cx: FunctionContext) -> JsResult<JsObject> {
    
}

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


    // register_module!( mut cx, {
    //     cx.export_function("find_matches", find_matches)?;
    //     // cx.export_class::<JsAhoCorasick>("AhoCorasick")?;
    //     Ok(())
    // });

        register_module!( mut cx, {
        cx.export_function("find_matches", find_matches)?;
        Ok(())
    });