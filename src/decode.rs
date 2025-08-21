use crate::instruction::Instruction;


#[derive(Copy, Clone, Debug)]
pub struct ArmDecoder(u32);
impl ArmDecoder {
    pub fn new(instruction_word: u32) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instruction {
        todo!()
    }
}



#[derive(Copy, Clone, Debug)]
pub struct ThumbDecoder(u16);
impl ThumbDecoder {
    pub fn new(instruction_word: u16) -> Self {
        Self(instruction_word)
    }

    pub fn decode(self) -> Instruction {
        todo!()
    }
}
