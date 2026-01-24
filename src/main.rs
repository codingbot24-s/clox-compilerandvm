use std::{char, env, io, os::unix::fs::OpenOptionsExt};

#[derive(Debug)]
enum OPCODE {
    OPCONSTANT,
    OPNEGATE,
    OPRETURN,
    OPADD,
    OPSUB,
    OPMUL,
    OPDIVIDE,
}

// OPCODE --> u8
impl From<OPCODE> for u8 {
    fn from(value: OPCODE) -> Self {
        value as u8
    }
}

// u8 --> OPCODE

impl From<u8> for OPCODE {
    fn from(value: u8) -> Self {
        match value {
            0 => OPCODE::OPCONSTANT,
            1 => OPCODE::OPNEGATE,
            2 => OPCODE::OPRETURN,
            3 => OPCODE::OPADD,
            4 => OPCODE::OPSUB,
            5 => OPCODE::OPMUL,
            6 => OPCODE::OPDIVIDE,
            _ => unimplemented!(),
        }
    }
}

type Value = f64;
struct ValueArr {
    values: Vec<Value>,
}

impl ValueArr {
    fn new() -> Self {
        ValueArr { values: Vec::new() }
    }

    fn write_value(&mut self, value: Value) -> usize {
        let count = self.values.len();
        self.values.push(value);
        count
    }

    fn print_value(&self, which: usize) {
        print!("{}", self.values[which])
    }

    fn read_value(&self, which: usize) -> Value {
        self.values[which]
    }

    fn free(&mut self) {
        self.values = Vec::new();
    }
}

struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: ValueArr,
}

impl Chunk {
    fn new() -> Self {
        Chunk {
            code: Vec::new(),
            lines: Vec::new(),
            constants: ValueArr::new(),
        }
    }

    fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    fn write_opcode(&mut self, code: u8, line: usize) {
        self.lines.push(line);
        self.code.push(code);
    }
    fn disassemble_chunk(&self, name: &str) {
        println!("== {} == ", name);
        let mut offset = 0;
        while offset < self.code.len() {
            if let Ok(of) = self.disassemble_instruction(offset) {
                offset = of
            }
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> Result<usize, ()> {
        print!("{:04} ", offset);
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("  | ")
        } else {
            print!("{:4} ", self.lines[offset])
        }
        let code: OPCODE = self.code[offset].into();
        match code {
            OPCODE::OPCONSTANT => Ok(self.constant_instruction("OP_CONSTANT".to_string(), offset)),
            OPCODE::OPNEGATE => Ok(self.simple_instruction(&code, offset)),
            OPCODE::OPRETURN => Ok(self.simple_instruction(&code, offset)),
            OPCODE::OPADD => Ok(self.simple_instruction(&code, offset)),
            OPCODE::OPSUB => Ok(self.simple_instruction(&code, offset)),
            OPCODE::OPMUL => Ok(self.simple_instruction(&code, offset)),
            OPCODE::OPDIVIDE => Ok(self.simple_instruction(&code, offset)),
        }
    }

    fn simple_instruction(&self, v: &OPCODE, offset: usize) -> usize {
        println!("{:?}", v);
        offset + 1
    }

    fn constant_instruction(&self, name: String, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        print!("{:-16} {:?} '", name, constant);
        self.constants.print_value(constant as usize);
        println!("'");
        offset + 2
    }

    fn read_chunk_byte(&self, ip: usize) -> u8 {
        self.code[ip]
    }

    fn free(&mut self) {
        self.code = Vec::new();
        self.constants = ValueArr::new();
    }
    // write the value in the value arr and get the count
    fn add_constants(&mut self, value: Value) -> usize {
        self.constants.write_value(value)
    }

    fn get_constant(&self, index: usize) -> Value {
        self.constants.read_value(index)
    }
}

enum INTERPRETRESULT {
    INTERPRETOK,
    INTERPRETCOMPILEERROR,
    INTERPRETRUNTIMEERROR,
}

/*
        VM
*/

struct Vm {
    ip: usize,
    stack: Vec<Value>,
}

impl Vm {
    fn new_vm() -> Self {
        Vm {
            ip: 0,
            stack: Vec::with_capacity(256),
        }
    }

    fn interpret(&mut self, source: String) -> INTERPRETRESULT {
        let compiler = compiler::new();
        compiler.compile(&source);
        INTERPRETRESULT::INTERPRETOK
    }

    fn run(&mut self, c: &Chunk) -> INTERPRETRESULT {
        loop {
            #[cfg(feature = "DEBUG_TRACE_EXECUTION")]
            {
                print!("          ");
                for v in &self.stack {
                    print!("[  {v}  ]")
                }
                println!();
                c.disassemble_instruction(self.ip);
            }

            let ins = self.read_byte(c);
            match ins {
                OPCODE::OPRETURN => {
                    println!("{}", self.stack.pop().unwrap());
                    return INTERPRETRESULT::INTERPRETOK;
                }
                OPCODE::OPCONSTANT => {
                    let v = self.read_constants(c);
                    self.stack.push(v);
                    println!("value is {}", v);
                }
                OPCODE::OPNEGATE => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(-value);
                }
                // call the binary op function according to case
                OPCODE::OPADD => {
                    self.binary_op(|a, b| a + b);
                }
                OPCODE::OPSUB => {
                    self.binary_op(|a, b| a - b);
                }
                OPCODE::OPMUL => {
                    self.binary_op(|a, b| a * b);
                }
                OPCODE::OPDIVIDE => {
                    self.binary_op(|a, b| a / b);
                }
            }
        }
    }

    fn read_byte(&mut self, c: &Chunk) -> OPCODE {
        // get the instruc`tion from the chunk via ip
        let ins: OPCODE = c.read_chunk_byte(self.ip).into();
        self.ip += 1;
        ins
    }

    fn read_constants(&mut self, c: &Chunk) -> Value {
        let index = c.read_chunk_byte(self.ip);
        self.ip += 1;
        c.get_constant(index as usize)
    }

    fn reset_stack(&mut self) {
        self.stack = Vec::new();
    }

    fn binary_op(&mut self, op: fn(a: Value, b: Value) -> Value) {
        let a = self.stack.pop().unwrap();
        let b = self.stack.pop().unwrap();
        self.stack.push(op(a, b));
    }

    fn free_vm(&mut self) {}
}

/*

    code running function
*/

fn repl(vm: &mut Vm) {
    println!(">   ");
    loop {
        for line in io::stdin().lines() {
            if let Ok(l) = line {
                let _ = vm.interpret(l);
            } else {
                println!()
            }
        }
    }
}

fn run_file(path: &str, vm: &mut Vm) {
    let buf = std::fs::read_to_string(path);
    match buf {
        Ok(source) => {
            vm.interpret(source);
        }
        Err(err) => {
            println!("error reading from the file")
        }
    }
}

/*
    compiler
*/
struct compiler {}

impl compiler {
    fn new() -> Self {
        Self {}
    }
    fn compile(&self, source: &String) {
        let mut s = Scanner::new(source);
        let mut line: usize = 0;
        loop {
            let token = s.scan_token();
            if token.line != line {
                println!("{:4}", token.line);
                line = token.line;
            } else {
                println!("   | ");
            }

            println!("{:?} '{}',{},", token.ttype, token.lexeme, token.line);

            // TODO: NOTE add break if token type == eof
            // fix this or it will not work expected
            // error partialeq missing for tokentype
            if token.ttype == TokenType::EOF {
               break;
            }
        }
    }
}

/*
    Scanner
*/

struct Scanner {
    source: Vec<char>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    fn new(source: &String) -> Self {
        Self {
            source: source.chars().collect(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;
        if self.is_at_end() {
            self.make_token(TokenType::EOF);
        }
        let c = self.advance();
        match c {
            '(' => self.make_token(TokenType::LEFTPAREN),
            ')' => self.make_token(TokenType::RIGHTPAREN),
            '{' => self.make_token(TokenType::LEFTBRACE),
            '}' => self.make_token(TokenType::RIGHTBRACE),
            ',' => self.make_token(TokenType::COMMA),
            '.' => self.make_token(TokenType::DOT),
            '-' => self.make_token(TokenType::MINUS),
            '+' => self.make_token(TokenType::PLUS),
            ';' => self.make_token(TokenType::SEMICOLON),
            '*' => self.make_token(TokenType::STAR),
            '/' => self.make_token(TokenType::SLASH),
            '!' => {
                if self.check_expected('=') {
                    self.make_token(TokenType::BANGEQUAL)
                } else {
                    self.make_token(TokenType::BANG)
                }
            }
            '=' => {
                if self.check_expected('=') {
                    self.make_token(TokenType::EQUALEQUAL)
                } else {
                    self.make_token(TokenType::EQUAL)
                }
            }
            '>' => {
                if self.check_expected('=') {
                    self.make_token(TokenType::GREATEREQUAL)
                } else {
                    self.make_token(TokenType::GREATER)
                }
            }
            '<' => {
                if self.check_expected('=') {
                    self.make_token(TokenType::LESSEQUAL)
                } else {
                    self.make_token(TokenType::LESS)
                }
            }
            '"' => self.string(),
            '0'..'9' => self.number(),
            _ if c.is_alphabetic() => self.identifier(),
            _ => self.error_token(&"Unexpected character."),
        }
    }

    fn identifier(&mut self) -> Token {
        while self.peek().is_alphabetic() || self.peek().is_digit(10) {
            self.advance();
        }
        return self.make_token(self.identifier_type());
    }

    fn identifier_type(&self) -> TokenType {
        match self.source[self.start] {
            'a' => self.check_keyword(1, 2, "nd", TokenType::AND),
            'c' => self.check_keyword(1, 4, "lass", TokenType::CLASS),
            'e' => self.check_keyword(1, 3, "lse", TokenType::ELSE),
            'f' => {
                if self.current - self.start > 1 {
                    match self.source[self.start + 1] {
                        'a' => self.check_keyword(2,3,"lse",TokenType::FALSE),
                        'o' => self.check_keyword(2,1,"r",TokenType::FOR),
                        'u' => self.check_keyword(2,1,"n",TokenType::FUN),
                        _ => TokenType::IDENTIFIER,
                    }
                }else {
                    TokenType::IDENTIFIER
                }
            }
            'i' => self.check_keyword(1, 1, "f", TokenType::IF),
            'n' => self.check_keyword(1, 2, "il", TokenType::NIL),
            'o' => self.check_keyword(1, 1, "or", TokenType::OR),
            'p' => self.check_keyword(1, 4, "rint", TokenType::PRINT),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::RETURN),
            's' => self.check_keyword(1, 4, "uper", TokenType::SUPER),
            't' => {
                    if self.current - self.start > 1 {
                    match self.source[self.start + 1] {
                        'h' => self.check_keyword(2,2,"is",TokenType::THIS),
                        'r' => self.check_keyword(2,2,"r",TokenType::TRUE),
                        _ => TokenType::IDENTIFIER,
                    }
                }else {
                    TokenType::IDENTIFIER
                }
            }
            'v' => self.check_keyword(1, 2, "ar", TokenType::VAR),
            'w' => self.check_keyword(1, 4, "hile", TokenType::WHILE),
            _ => TokenType::IDENTIFIER,
        }
    }

    fn check_keyword(&self,start:usize,len:usize,rest:&str,tt:TokenType) -> TokenType {
        // if self.current - self.start != start + len {

            //return TokenType::IDENTIFIER 
        // }
        // let compare:String = self.source[self.start + start..self.current].iter().collect();
        // if compare.as_str() == rest {
           //  return tt
        // }

        TokenType::IDENTIFIER
    }

    fn number(&mut self) -> Token {
        while self.peek().is_digit(10) {
            self.advance();
        }
        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }
        self.make_token(TokenType::NUMBER)
    }
    fn is_at_end(&self) -> bool {
        self.current == self.source.len()
    }

    fn make_token(&self, tt: TokenType) -> Token {
        Token {
            ttype: tt,
            lexeme: self.source[self.start..self.current].iter().collect(),
            line: self.line,
        }
    }
    fn error_token(&self, message: &str) -> Token {
        Token {
            ttype: TokenType::ERROR,
            lexeme: message.to_string(),
            line: self.line,
        }
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        // TODO: some problem here --> index out of bound 
        self.source[self.current - 1]
    }
    fn check_expected(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        } else if self.source[self.current] != expected {
            return false;
        }
        self.current += 1;
        return true;
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                    break;
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                    break;
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => {
                    return;
                }
            }
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        }else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        } else {
            self.source[self.current + 1]
        }
    }
    fn string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }
        if self.is_at_end() {
            return self.make_token(TokenType::EOF);
        }
        // for closing qoute'"'
        self.advance();
        self.make_token(TokenType::STRING)
    }
}

struct Token {
    ttype: TokenType,
    lexeme: String,
    line: usize,
}

impl Token {}

#[derive(Debug, PartialEq)]
enum TokenType {
    LEFTPAREN,
    RIGHTPAREN,
    LEFTBRACE,
    RIGHTBRACE,
    COMMA,
    DOT,
    MINUS,
    PLUS,
    SEMICOLON,
    SLASH,
    STAR,

    BANG,
    BANGEQUAL,
    EQUAL,
    EQUALEQUAL,
    GREATER,
    GREATEREQUAL,
    LESS,
    LESSEQUAL,

    IDENTIFIER,
    STRING,
    NUMBER,

    AND,
    CLASS,
    ELSE,
    FALSE,
    FUN,
    FOR,
    IF,
    NIL,
    OR,
    PRINT,
    RETURN,
    SUPER,
    THIS,
    TRUE,
    VAR,
    WHILE,

    EOF,

    ERROR,
}

fn main() {
    let mut vm = Vm::new_vm();

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&args[1], &mut vm),
        _ => {
            println!("Usage: clox [path]");
            std::process::exit(64);
        }
    }

    vm.free_vm();
}

// remaining from scanner //
