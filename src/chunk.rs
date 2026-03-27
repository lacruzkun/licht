#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    OpConstant(usize),
    OpReturn,
}

pub type Int = isize;

#[derive(Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    lines: Vec<usize>,
    lines_repeat: Vec<usize>,
    constants: Vec<Int>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: vec![],
            lines: vec![],
            lines_repeat: vec![],
            constants: vec![],
        }
    }

    pub fn write(&mut self, byte: OpCode, ln: usize) {
        self.code.push(byte);
        if self.lines.is_empty() {
            self.lines.push(ln);
            self.lines_repeat.push(1);
        } else {
            if self.lines[self.lines.len() - 1] == ln {
                self.lines_repeat[self.lines.len() - 1] += 1;
            } else {
                self.lines.push(ln);
                self.lines_repeat.push(1);
            }
        }
    }

    pub fn add_constant(&mut self, value: Int) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);

        for offset in 0..self.code.len() {
            self.disassemble_instruction(offset);
        }
    }

    pub fn get_line(&self, offset: usize) -> usize {
        if offset == 0 {
            return self.lines[0];
        }
        let mut index = 0;
        let mut line_no = 0;
        for lines in self.lines_repeat.clone() {
            line_no += lines;
            if line_no - 1 >= offset {
                break;
            }
            index += 1;
        }
        self.lines[index]
    }

    pub fn disassemble_instruction(&self, offset: usize) {
        print!("{:04} ", offset);

        let current_line = self.get_line(offset);

        if offset > 0 && current_line == self.get_line(offset - 1) {
            print!("{:2}| ", "");
        } else {
            print!("{} ", current_line);
        }

        let instruction = self.code[offset];

        match instruction {
            OpCode::OpReturn => {
                println!("OP_RETURN");
            }
            OpCode::OpConstant(offset) => {
                println!("OP_CONSTANT {:04} {}", offset, self.constants[offset]);
            }
        }
    }
}
