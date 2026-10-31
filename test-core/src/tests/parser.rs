use crate::assert_ok;
use symbol_table::static_symbol;
use tau_core::{
    ast::{
        Identifier,
        declaration::{DeclFunc, DeclStruct, FuncModifiers},
        origin::{ExprOrigin, Literal},
        prelude::*,
    },
    lexer::{Lexer, Source},
    parser::Parser,
    type_::prelude::*,
};

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
      };
    }
  }
  "#;
    let lexer = Lexer::new(source, file_id);
    let ast = Parser::new(lexer).parse();
    assert_ok!(&ast);
    let expected = vec![
        struct_(
            DeclStruct::new(
                Identifier::new(static_symbol!("Structure"), Source::new(file_id, 10..19)),
                vec![
                    (
                        Identifier::new(static_symbol!("ptr"), Source::new(file_id, 26..28)),
                        ptr(named(static_symbol!("Structure"), vec![]), true),
                    ),
                    (
                        Identifier::new(static_symbol!("array"), Source::new(file_id, 47..51)),
                        array(
                            array(array(named(static_symbol!("Structure"), vec![]), 3), 3),
                            3,
                        ),
                    ),
                ],
            ),
            Source::new(file_id, 3..86),
        ),
        func(
            DeclFunc::new(
                Identifier::new(static_symbol!("modifiers"), Source::new(file_id, 100..108)),
                void(),
                vec![],
                vec![],
                FuncModifiers::new(true, true),
            ),
            Source::new(file_id, 87..117),
        ),
        func(
            DeclFunc::new(
                Identifier::new(static_symbol!("keywords"), Source::new(file_id, 121..128)),
                named(static_symbol!("T3"), vec![]),
                vec![
                    (
                        Identifier::new(static_symbol!("p1"), Source::new(file_id, 130..131)),
                        named(static_symbol!("T1"), vec![]),
                    ),
                    (
                        Identifier::new(static_symbol!("p2"), Source::new(file_id, 138..139)),
                        named(static_symbol!("T2"), vec![]),
                    ),
                ],
                vec![
                    let_(
                        Identifier::new(static_symbol!("a"), Source::new(file_id, 162..163)),
                        None,
                        literal(
                            ExprOrigin::new(Literal::Bool(true), Source::new(file_id, 166..169)),
                            (),
                        ),
                        Source::new(file_id, 158..169),
                    ),
                    while_(
                        variable(
                            Identifier::new(static_symbol!("a"), Source::new(file_id, 182..182)),
                            (),
                        ),
                        block(
                            vec![expr_stmt(
                                if_(
                                    variable(
                                        Identifier::new(
                                            static_symbol!("a"),
                                            Source::new(file_id, 196..196),
                                        ),
                                        (),
                                    ),
                                    break_(Source::new(file_id, 209..221)),
                                    Some(return_(
                                        call(
                                            variable(
                                                Identifier::new(
                                                    static_symbol!("call"),
                                                    Source::new(file_id, 246..249),
                                                ),
                                                (),
                                            ),
                                            vec![],
                                            Source::new(file_id, 246..251),
                                            (),
                                        ),
                                        Source::new(file_id, 239..259),
                                    )),
                                    Source::new(file_id, 193..260),
                                    (),
                                ),
                                Source::new(file_id, 193..266),
                            )],
                            Source::new(file_id, 182..270),
                        ),
                        Source::new(file_id, 176..270),
                    ),
                ],
                FuncModifiers::new(false, false),
            ),
            Source::new(file_id, 118..274),
        ),
    ];
    assert_eq!(ast.unwrap(), expected)
}
