use crate::assert_ok;
use pretty_assertions::assert_eq;
use symbol_table::static_symbol;
use tau_core::{
    ast::{
        Identifier,
        declaration::{FuncDecl, FuncModifiers, StructDecl},
        lookup::{CallLookup, Lookup, VarLookup},
        origin::{Literal, Origin},
        prelude::*,
    },
    lexer::{Lexer, Source},
    parser::Parser,
    type_::prelude::*,
};
use pretty_assertions::assert_eq;

#[test]
fn parses_from_lexer() {
    let file_id = 0;
    let source = r#"
  struct Structure {
    ptr: *Structure,
    array: [[[Structure; 3]; 3]; 3],
  }

  extern io fn modifiers();
  
  fn keywords(p1: T1, p2: T2) -> T3 {
    let a = true;
    while(a) {
      if(a) {
        break;
      } else {
        return call();
      }
    }
  }
  "#;
    let lexer = Lexer::new(source, file_id);
    let ast = Parser::new(lexer).parse();
    assert_ok!(&ast);
    let expected = vec![
        struct_(
            StructDecl::new(
                Identifier::new(static_symbol!("Structure"), Source::new(file_id, 10..19)),
                vec![
                    (
                        Identifier::new(static_symbol!("ptr"), Source::new(file_id, 26..29)),
                        ptr(named(static_symbol!("Structure"), vec![]), true),
                    ),
                    (
                        Identifier::new(static_symbol!("array"), Source::new(file_id, 47..52)),
                        array(
                            array(array(named(static_symbol!("Structure"), vec![]), 3), 3),
                            3,
                        ),
                    ),
                ],
            ),
            Source::new(file_id, 3..83),
        ),
        func(
            FuncDecl::new(
                Identifier::new(static_symbol!("modifiers"), Source::new(file_id, 100..109)),
                void(),
                vec![],
                vec![],
                FuncModifiers::new(true, true),
            ),
            Source::new(file_id, 87..112),
        ),
        func(
            FuncDecl::new(
                Identifier::new(static_symbol!("keywords"), Source::new(file_id, 121..129)),
                named(static_symbol!("T3"), vec![]),
                vec![
                    (
                        Identifier::new(static_symbol!("p1"), Source::new(file_id, 130..132)),
                        named(static_symbol!("T1"), vec![]),
                    ),
                    (
                        Identifier::new(static_symbol!("p2"), Source::new(file_id, 138..140)),
                        named(static_symbol!("T2"), vec![]),
                    ),
                ],
                vec![
                    let_(
                        Identifier::new(static_symbol!("a"), Source::new(file_id, 162..163)),
                        None,
                        literal(
                            Origin::new(Literal::Bool(true), Source::new(file_id, 166..170)),
                            (),
                        ),
                        Source::new(file_id, 158..170),
                    ),
                    while_(
                        variable(
                            VarLookup::new(
                                Identifier::new(
                                    static_symbol!("a"),
                                    Source::new(file_id, 182..183),
                                ),
                                Lookup::Unqualified,
                            ),
                            (),
                        ),
                        vec![if_(
                            variable(
                                VarLookup::new(
                                    Identifier::new(
                                        static_symbol!("a"),
                                        Source::new(file_id, 196..197),
                                    ),
                                    Lookup::Unqualified,
                                ),
                                (),
                            ),
                            vec![break_(Source::new(file_id, 209..215))],
                            Some(vec![return_(
                                Some(call(
                                    CallLookup::unqualified(Identifier::new(
                                        static_symbol!("call"),
                                        Source::new(file_id, 246..250),
                                    )),
                                    vec![],
                                    Source::new(file_id, 246..252),
                                    (),
                                )),
                                Source::new(file_id, 239..253),
                            )]),
                            Source::new(file_id, 193..261),
                        )],
                        Source::new(file_id, 176..267),
                    ),
                ],
                FuncModifiers::new(false, false),
            ),
            Source::new(file_id, 118..271),
        ),
    ];
    assert_eq!(ast.unwrap(), expected)
}
