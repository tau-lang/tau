use crate::assert_ok;
use bpaf::doc::Section;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
use tau_cli::options::options;

const MANPAGE_FOOTER: &'static str = r#"
.SH DESCRIPTION
The \fBtau\fR command takes a single Tau files and compiles it to target (e.g. C).

.SH EXAMPLES
tau example/main.tau

.SH REPORTING BUGS
You can report bugs on Codeberg <https://codeberg.org/tau-lang/tau/issues>

.SH SEE ALSO
ochtendzon(1)

.SH COPYRIGHT
Licensed under the EUPL
"#;

// CI will ensure that documentation is always up to date
#[test]
fn update_doc() {
    let options = options();
    let mut man = options.render_manpage(
        "tau",
        Section::General,
        Some("31 Oct 2026"),
        Some("tau-lang"),
        Some("tau"),
    );
    man.push_str(MANPAGE_FOOTER);
    let mut expected_man = String::new();
    let path = dbg!(
        assert_ok!(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../man/tau.1")
                .canonicalize()
        )
        .expect("path exists")
    );
    let mut file = assert_ok!(File::open(&path)).expect("could open file");
    assert_ok!(file.read_to_string(&mut expected_man)).expect("could read file");
    fs::write(path, &man).expect("could write file");

    // This will fail whenever we forgot to update the docs.
    assert_eq!(man, expected_man)
}
