use crate::assert_ok;
use bpaf::doc::Section;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};
use tau_cli::options::options;

const MANPAGE_FOOTER: &str = r#"
.SH DESCRIPTION
The \fBtau\fR command compiles a Tau source file and generates code using the
specified code generator (e.g., C). Generated files are written to the output
directory, with file extensions determined automatically by the code generator.

If no output directory is specified, the current working directory is used.
If no output name is specified, the input file name is used.

.SH EXAMPLES
Compile a Tau source file:

.EX
tau example/main.tau
.EE

.SH REPORTING BUGS
Report bugs at:
.UR https://codeberg.org/tau-lang/tau/issues
https://codeberg.org/tau-lang/tau/issues
.UE

.SH SEE ALSO
ochtendzon(1)

.SH COPYRIGHT
Licensed under the European Union Public Licence (EUPL).

The full license text is available at:
.UR https://codeberg.org/tau-lang/tau/src/branch/main/LICENSE
https://codeberg.org/tau-lang/tau/src/branch/main/LICENSE
.UE
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
