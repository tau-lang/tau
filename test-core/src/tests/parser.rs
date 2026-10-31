use crate::assert_ok;
use symbol_table::static_symbol;
use tau_core::{
    ast::{Identifier, declaration::DeclStruct, prelude::*},
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
  
  fn keywords() {
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
    let expected = vec![struct_(
        DeclStruct::new(
            Identifier::new(static_symbol!("Structure"), Source::new(0, 10..20)),
            vec![
                (
                    Identifier::new(static_symbol!("ptr"), Source::new(0, 26..29)),
                    ptr(named(static_symbol!("Structure"), vec![]), true),
                ),
                (
                    Identifier::new(static_symbol!("array"), Source::new(0, 47..52)),
                    array(
                        array(array(named(static_symbol!("Structure"), vec![]), 3), 3),
                        3,
                    ),
                ),
            ],
        ),
        Source::new(0, 3..87),
    )];
    assert_eq!(ast.unwrap(), expected)
}
