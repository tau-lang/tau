use std::rc::Rc;

impl Ast {
    pub fn r(self) -> Rc<Self> {
        Rc::new(self)
    }
}

#[derive(Debug)]
pub enum Ast {
    Number(i64),
    Id(Id),
    Not {
        term: Rc<Ast>,
    },
    Equal {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    NotEqual {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Add {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Subtract {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Multiply {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Defive {
        lhs: Rc<Ast>,
        rhs: Rc<Ast>,
    },
    Call {
        callee: Id,
        args: Vec<Ast>,
    },
    Return {
        term: Rc<Ast>,
    },
    Block {
        terms: Vec<Ast>,
    },
    If {
        conditional: Rc<Ast>,
        consequence: Rc<Ast>,
        alternative: Rc<Ast>,
    },
    Function {
        name: Id,
        parameters: Vec<String>,
        body: Rc<Ast>,
    },
    Var {
        name: String,
        value: Rc<Ast>,
    },
    Assignment {
        name: String,
        value: Rc<Ast>,
    },
    While {
        conditional: Rc<Ast>,
        body: Rc<Ast>,
    },
}

#[derive(Debug)]
pub struct Id {
    pub id: String,
}

impl Id {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}
