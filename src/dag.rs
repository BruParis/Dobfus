use crate::error::DAGError;
use std::collections::VecDeque;
use std::fmt::{self, Debug, Display, Formatter};

pub trait DAGTrait: std::fmt::Debug {
    fn is_cst(&self) -> bool;
    fn is_mba(&self) -> bool;
    fn is_mba_term(&self) -> bool;
    fn bitwise(&self) -> bool;
    fn valid(&self) -> bool;
}

impl DAGTrait for Box<dyn DAGTrait> {
    fn is_cst(&self) -> bool {
        self.as_ref().is_cst()
    }
    fn is_mba(&self) -> bool {
        self.as_ref().is_mba()
    }
    fn is_mba_term(&self) -> bool {
        self.as_ref().is_mba_term()
    }
    fn valid(&self) -> bool {
        self.as_ref().valid()
    }

    fn bitwise(&self) -> bool {
        self.as_ref().bitwise()
    }
}

#[derive(Debug)]
pub enum DAGValue {
    U32(u32),
    Var(char),
}

impl Display for DAGValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DAGValue::U32(u) => write!(f, "{}", u),
            DAGValue::Var(c) => write!(f, "{}", c),
        }
    }
}

struct DAGLeaf {
    value: DAGValue,
    sign: bool,
}

impl DAGLeaf {
    pub fn new(value: DAGValue, sign: bool) -> Self {
        DAGLeaf { value, sign }
    }
}

impl Debug for DAGLeaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sign {
            return write!(f, "{}", self.value);
        } else {
            return write!(f, "~{}", self.value);
        }
    }
}

impl DAGTrait for DAGLeaf {
    fn is_cst(&self) -> bool {
        match self.value {
            DAGValue::U32(u) => true,
            DAGValue::Var(c) => false,
        }
    }
    fn is_mba(&self) -> bool {
        true
    }
    fn is_mba_term(&self) -> bool {
        true
    }
    fn valid(&self) -> bool {
        true
    }
    fn bitwise(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct ExpVar {
    var: char,
}

struct DAGNode {
    op: char,
    ch: Box<Vec<Box<dyn DAGTrait>>>,
    sign: bool,
}

impl DAGNode {
    pub fn new(op: char, sign: bool) -> Result<Self, DAGError> {
        Ok(DAGNode {
            op,
            ch: Box::new(Vec::new()),
            sign,
        })
    }

    fn push_ch(&mut self, ch: Box<dyn DAGTrait>) {
        self.ch.push(ch);
    }

    fn push_ch_vec(&mut self, ch: &mut Vec<Box<dyn DAGTrait>>) {
        self.ch.append(ch);
    }
}

impl DAGTrait for DAGNode {
    fn is_cst(&self) -> bool {
        self.ch.iter().all(|ch| ch.is_cst())
    }
    fn is_mba(&self) -> bool {
        match self.op {
            '+' => self.ch.iter().all(|ch| ch.is_mba_term()),
            '.' => self.is_mba_term(),
            _ => self.ch.iter().all(|ch| ch.bitwise()),
        }
    }

    fn is_mba_term(&self) -> bool {
        match self.op {
            '+' => false,
            '.' => {
                let mut node_count = 0;
                return self.ch.iter().all(move |ch| {
                    if !ch.is_cst() {
                        node_count += 1;
                    }

                    if node_count > 1 {
                        return false;
                    }

                    return ch.bitwise();
                });
            }
            _ => self.ch.iter().all(|ch| ch.bitwise()),
        }
    }

    fn valid(&self) -> bool {
        if self.ch.len() < 2 {
            return false;
        }

        self.ch.iter().all(|ch| ch.valid())
    }
    fn bitwise(&self) -> bool {
        if "+.".contains(self.op) {
            return false;
        }
        self.ch.iter().all(|ch| ch.bitwise())
    }
}

impl fmt::Debug for DAGNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sign {
            write!(f, "{}", self.op);
        } else {
            write!(f, "~{}", self.op);
        }

        let mut it_ch = self.ch.iter().peekable();
        while let Some(ch) = it_ch.next() {
            if it_ch.peek().is_none() {
                write!(f, "{:#?}", ch);
            } else {
                write!(f, "{:#?};", ch);
            }
        }

        write!(f, "/");

        Ok(())
    }
}

pub struct DAGFactory;
impl DAGFactory {
    pub fn new_dag(rpn: &mut VecDeque<String>) -> Result<Box<dyn DAGTrait>, DAGError> {
        let node = DAGFactory::build_dag(rpn)?;

        if !node.valid() {
            return Err(DAGError::RPNSyntaxError());
        }

        Ok(node)
    }
    fn build_dag(rpn: &mut VecDeque<String>) -> Result<Box<dyn DAGTrait>, DAGError> {
        fn take_node_stack(
            curr_node: &mut Option<Box<DAGNode>>,
            node_stack: &mut VecDeque<Box<DAGNode>>,
            pop_stack: bool,
        ) {
            if let Some(mut node) = curr_node.take() {
                if let (true, Some(par_node)) = (node.ch.len() > 1, node_stack.back_mut()) {
                    if par_node.op == node.op {
                        par_node.push_ch_vec(&mut node.ch);
                    } else {
                        par_node.push_ch(node);
                    }

                    if pop_stack {
                        *curr_node = node_stack.pop_back();
                    }
                } else {
                    node_stack.push_back(node);
                }
            }
        }
        println!("{}", format!("rpn: {:?}", rpn));

        if rpn.len() == 0 {
            return Err(DAGError::RPNEmpty());
        }

        let mut prev_leaf = false;
        let mut curr_node: Option<Box<DAGNode>> = None;
        let mut curr_sign = true;
        let mut node_stack: VecDeque<Box<DAGNode>> = VecDeque::new();

        while let Some(elem) = rpn.pop_back() {
            println!(
                "{}",
                format!(
                    "{:?} - node_stack: {:?} - curr_node: {:?} - elem: {}",
                    rpn, node_stack, curr_node, elem
                )
            );
            match elem.as_str() {
                "+" | "-" | "." | "^" | "&" | "|" => {
                    prev_leaf = false;
                    if let Some(new_op) = elem.chars().next() {
                        if let Some(node) = curr_node.as_deref() {
                            if node.op == new_op {
                                continue;
                            }
                        }

                        take_node_stack(&mut curr_node, &mut node_stack, false);

                        curr_node = Some(Box::new(DAGNode::new(new_op, curr_sign)?));
                    } else {
                        unreachable!()
                    }

                    curr_sign = true;
                }
                "~" => {
                    if !curr_sign {
                        return Err(DAGError::RPNSyntaxError());
                    }
                    curr_sign = false;
                }
                _ => {
                    let leaf: Box<dyn DAGTrait>;
                    if let Ok(term_u) = elem.parse::<u32>() {
                        leaf = Box::new(DAGLeaf::new(DAGValue::U32(term_u), curr_sign));
                    } else if let (true, Some(c_var)) = (elem.len() == 1, elem.chars().next()) {
                        leaf = Box::new(DAGLeaf::new(DAGValue::Var(c_var), curr_sign));
                    } else {
                        return Err(DAGError::RPNSyntaxError());
                    }

                    match curr_node.as_mut() {
                        Some(node) => node.push_ch(leaf),
                        None => {
                            if rpn.len() == 0 {
                                return Ok(leaf);
                            }
                            return Err(DAGError::RPNSyntaxError());
                        }
                    }

                    if prev_leaf {
                        take_node_stack(&mut curr_node, &mut node_stack, true);
                    }

                    curr_sign = true;
                    prev_leaf = true;
                }
            }
        }

        println!(
            "-> {}",
            format!(
                "{:?} - node_stack: {:?} - curr_node: {:?}",
                rpn, node_stack, curr_node
            )
        );

        if let Some(node) = curr_node {
            return Ok(node);
        }

        if let (true, Some(node)) = (node_stack.len() == 1, node_stack.pop_back()) {
            return Ok(node);
        }

        Err(DAGError::RPNSyntaxError())
    }
}