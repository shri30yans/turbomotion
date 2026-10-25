//! Opt-in integration against an existing local TypeScript compiler; never downloads packages.
mod common;
use std::{fs,path::Path};
#[test]
#[ignore = "set TURBOMOTION_TEST_TSC to an installed tsc executable or tsc.js"]
fn actual_typescript_compiler_checks_types_without_executing_the_film() {
    let compiler=std::env::var_os("TURBOMOTION_TEST_TSC").expect("set TURBOMOTION_TEST_TSC");
    let dir=common::scratch("actual-typecheck","tsc");
    let file=dir.join("film.ts");
    let sdk=Path::new(env!("CARGO_MANIFEST_DIR")).join("sdk");
    fs::write(&file,"import {comp} from 'turbomotion'; throw new Error('must never execute'); export default comp({width:100,height:100,fps:30});").unwrap();
    let report=turbomotion_script::check::check(&file,Some(&sdk),Path::new(&compiler)).unwrap();
    assert!(report.success,"{}",report.diagnostics);
    fs::write(&file,"import {comp} from 'turbomotion'; export default comp({width:'wrong',height:100,fps:30});").unwrap();
    let report=turbomotion_script::check::check(&file,Some(&sdk),Path::new(&compiler)).unwrap();
    assert!(!report.success);
    assert!(report.diagnostics.contains("not assignable to type 'number'"),"{}",report.diagnostics);
}
