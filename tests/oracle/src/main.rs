use std::{fs::read_to_string, path::PathBuf, process::Command};

use cfg_if::cfg_if;
use lang_tester::LangTester;
use run_script::{IoOptions, ScriptOptions};
use tempfile::TempDir;

fn main() {
    static COMMENT_PREFIX: &str = ";;";
    let tempdir = TempDir::new().unwrap();
    // build runtime

    let options = ScriptOptions {
        output_redirection: IoOptions::Inherit,
        ..ScriptOptions::new()
    };
    let args = vec![];
    run_script::run(
        r#"
            (cd ../../runtime && cargo build)
            (cd ../../cli && cargo build)
            mv ../../target/debug/pseudo ${CARGO_HOME}/bin
            "#,
        &args,
        &options,
    )
    .unwrap();

    cfg_if! {
        if #[cfg(target_os="macos")] {
            let mut copy_to = tempdir.path().to_path_buf();
            copy_to.push("libruntime.dylib");

            std::fs::copy("../../target/debug/libruntime.dylib", copy_to).expect("failed to copy runtime");
        } else if #[cfg(target_os="linux")] {
            let mut copy_to = tempdir.path().to_path_buf();
            copy_to.push("libruntime.so");

            std::fs::copy("../../target/debug/libruntime.so", copy_to).expect("failed to copy runtime");
        } else {
            compile_error!("unsupported target");
        }
    }

    LangTester::new()
        .test_dir("../../examples")
        .test_file_filter(|p| p.extension().unwrap().to_str().unwrap() == "pseudo")
        .test_extract(|p| {
            read_to_string(p)
                .unwrap()
                .lines()
                .skip_while(|l| !l.starts_with(COMMENT_PREFIX))
                .take_while(|l| l.starts_with(COMMENT_PREFIX))
                .map(|l| &l[COMMENT_PREFIX.len()..])
                .collect::<Vec<_>>()
                .join("\n")
        })
        .test_cmds(move |p| {
            let mut exe = PathBuf::new();
            exe.push(&tempdir);
            exe.push(p.file_stem().unwrap());

            let mut with_o = exe.clone();
            with_o.set_extension("o");
            let mut compiler = Command::new("pseudo");
            compiler.args(&[p.to_str().unwrap(), with_o.as_path().to_str().unwrap()]);

            let run = Command::new(exe.clone());

            cfg_if! {
                if #[cfg(target_os="macos")] {
                    let mut libruntime_loc = PathBuf::new();
                    libruntime_loc.push(&tempdir);
                    libruntime_loc.push("libruntime.dylib");
                } else if #[cfg(target_os="linux")] {
                    let mut libruntime_loc = PathBuf::new();
                    libruntime_loc.push(&tempdir);
                    libruntime_loc.push("libruntime.so");
                } else {
                    compile_error!("invalid target");
                }
            };

            let mut link = Command::new("cc");
            link.args(&[
                with_o.as_path().to_str().unwrap(),
                libruntime_loc.as_path().to_str().unwrap(),
                "-o",
                exe.as_path().to_str().unwrap(),
            ]);
            vec![("compiler", compiler), ("linker", link), ("run", run)]
        })
        .run();
}
