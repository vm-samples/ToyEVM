//! EVM instance
//!
//! EthereumにおけるEVMインスタンスを管理するモジュール

extern crate ethereum_types;
extern crate hex;

use super::state;
use super::util;
use ethereum_types::{H160, U256};
use util::not_implement_panic;

/// トランザクション実行に必要な環境変数
pub struct Environment {
    code_owner: H160, // 実行するコントラクトのオーナー
    sender: H160,     // トランザクションの送信者
    gas_price: usize, // gasのETHレート
    value: usize,     // トランザクションに添付されたEth
    code: Vec<u8>,    // 実行されるEVMバイトコード
    input: Vec<u8>,   // トランザクションに渡されるデータ(solidityでは引数として渡される)
}

impl Environment {
    pub fn new(code_owner: H160, sender: H160, gas_price: usize, value: usize) -> Self {
        return Self {
            code_owner,
            sender,
            gas_price,
            value,
            code: Default::default(),
            input: Default::default(),
        };
    }

    /// コードをセットする
    pub fn set_code(&mut self, code: Vec<u8>) {
        self.code = code;
    }

    /// インプットデータをセットする
    pub fn set_input(&mut self, input: Vec<u8>) {
        self.input = input;
    }
}

/// EVMインスタンス
pub struct VM {
    env: Environment, // 環境変数
    pc: usize,        // Program Counter
    gas: usize,       // gas残量
    sp: usize,        // スタックポインタ
    stack: Vec<U256>, // トランザクションのライフサイクルの間保持される一時的なスタック領域
    memory: Vec<u8>,  // トランザクションのライフサイクルの間保持される一時的なメモリ領域
    asm: Vec<String>, // 実行した命令を入れておく 逆アセンブルに利用
    returns: Vec<u8>, // アクションの返り値
}

/// Opcodeの実行で使われる汎用的な関数を実装している
impl VM {
    pub fn new(env: Environment) -> Self {
        let gas = env.value / env.gas_price;

        Self {
            env,
            pc: 0,
            gas,
            sp: 0,
            stack: Default::default(),
            memory: Default::default(),
            asm: Default::default(),
            returns: Default::default(),
        }
    }

    /// スタックへのpush
    fn push(&mut self, value: U256) {
        self.stack.push(value);
        self.sp += 1;
    }

    /// スタックからのpop
    fn pop(&mut self) -> U256 {
        let value = self.stack.pop().unwrap();
        self.sp -= 1;
        return value;
    }

    /// EVMバイトコードを1命令実行する
    fn exec(&mut self, contract: &mut state::AccountState) -> bool {
        let opcode = self.env.code[self.pc];
        self.pc += 1;

        // opcodeに対応するハンドラを呼び出す
        match opcode {
            // 0x00
            0x00 => self.op_stop(),
            0x01 => self.op_add(),
            0x02 => self.op_mul(),
            0x03 => self.op_sub(),
            0x04 => self.op_div(),
            0x05 => self.op_sdiv(),
            0x06 => self.op_mod(),
            0x07 => self.op_smod(),
            0x08 => self.op_addmod(),
            0x09 => self.op_mulmod(),
            0x0a => self.op_exp(),
            0x0b => self.op_sig_next_end(),
            // 0x10
            0x10 => self.op_lt(),
            0x11 => self.op_gt(),
            0x12 => self.op_slt(),
            0x13 => self.op_sgt(),
            0x14 => self.op_eq(),
            0x15 => self.op_is_zero(),
            0x16 => self.op_and(),
            0x17 => self.op_or(),
            0x18 => self.op_xor(),
            0x19 => self.op_not(),
            0x1a => self.op_byte(),
            // 0x20
            0x20 => self.op_sha3(),
            // 0x30
            0x30 => self.op_address(),
            0x31 => self.op_balance(),
            0x32 => self.op_origin(),
            0x33 => self.op_caller(),
            0x34 => self.op_callvalue(),
            0x35 => self.op_calldataload(),
            0x36 => self.op_calldatasize(),
            0x37 => self.op_calldatacopy(),
            0x38 => self.op_codesize(),
            0x39 => self.op_codecopy(),
            0x3a => self.op_gasprice(),
            0x3b => self.op_extcodesize(),
            0x3c => self.op_extcodecopy(),
            0x3d => self.op_returndatasize(),
            0x3e => self.op_returndatacopy(),
            0x3f => self.op_extcodehash(),
            // 0x40
            0x40 => self.op_blockhash(),
            0x41 => self.op_coinbase(),
            0x42 => self.op_timestamp(),
            0x43 => self.op_number(),
            0x44 => self.op_difficulty(),
            0x45 => self.op_gaslimit(),
            // 0x50
            0x50 => self.op_pop(),
            0x51 => self.op_mload(),
            0x52 => self.op_mstore(),
            0x54 => self.op_sload(contract),
            0x55 => self.op_sstore(contract),
            0x56 => self.op_jump(),
            0x57 => self.op_jumpi(),
            0x58 => self.op_pc(),
            0x59 => self.op_msize(),
            0x5a => self.op_gas(),
            0x5b => self.op_jumpdest(),
            // 0x60, 0x70
            0x60 => self.op_push(1),
            0x61 => self.op_push(2),
            0x62 => self.op_push(3),
            0x63 => self.op_push(4),
            0x64 => self.op_push(5),
            0x65 => self.op_push(6),
            0x66 => self.op_push(7),
            0x67 => self.op_push(8),
            0x68 => self.op_push(9),
            0x69 => self.op_push(10),
            0x6a => self.op_push(11),
            0x6b => self.op_push(12),
            0x6c => self.op_push(13),
            0x6d => self.op_push(14),
            0x6e => self.op_push(15),
            0x6f => self.op_push(16),
            0x70 => self.op_push(17),
            0x71 => self.op_push(18),
            0x72 => self.op_push(19),
            0x73 => self.op_push(20),
            0x74 => self.op_push(21),
            0x75 => self.op_push(22),
            0x76 => self.op_push(23),
            0x77 => self.op_push(24),
            0x78 => self.op_push(25),
            0x79 => self.op_push(26),
            0x7a => self.op_push(27),
            0x7b => self.op_push(28),
            0x7c => self.op_push(29),
            0x7d => self.op_push(30),
            0x7e => self.op_push(31),
            0x7f => self.op_push(32),
            // 0x80
            0x80 => self.op_dup(1),
            0x81 => self.op_dup(2),
            0x82 => self.op_dup(3),
            0x83 => self.op_dup(4),
            0x84 => self.op_dup(5),
            0x85 => self.op_dup(6),
            0x86 => self.op_dup(7),
            0x87 => self.op_dup(8),
            0x88 => self.op_dup(9),
            0x89 => self.op_dup(10),
            0x8a => self.op_dup(11),
            0x8b => self.op_dup(12),
            0x8c => self.op_dup(13),
            0x8d => self.op_dup(14),
            0x8e => self.op_dup(15),
            0x8f => self.op_dup(16),
            // 0x90
            0x90 => self.op_swap(1),
            0x91 => self.op_swap(2),
            0x92 => self.op_swap(3),
            0x93 => self.op_swap(4),
            0x94 => self.op_swap(5),
            0x95 => self.op_swap(6),
            0x96 => self.op_swap(7),
            0x97 => self.op_swap(8),
            0x98 => self.op_swap(9),
            0x99 => self.op_swap(10),
            0x9a => self.op_swap(11),
            0x9b => self.op_swap(12),
            0x9c => self.op_swap(13),
            0x9d => self.op_swap(14),
            0x9e => self.op_swap(15),
            0x9f => self.op_swap(16),
            // 0xa0
            0xa0 => self.op_log0(),
            0xa1 => self.op_log1(),
            0xa2 => self.op_log2(),
            0xa3 => self.op_log3(),
            0xa4 => self.op_log4(),
            // 0xf0
            0xf0 => self.op_create(),
            0xf1 => self.op_call(),
            0xf2 => self.op_callcode(),
            0xf3 => self.op_return(),
            0xf4 => self.op_delegatecall(),
            0xf5 => self.op_create2(),
            0xfa => self.op_staticcall(),
            0xfd => self.op_revert(),
            0xff => self.op_selfdestruct(),
            _ => not_implement_panic(),
        }

        // トランザクションを終了させるかのフラグ returnのみtrue
        return match opcode {
            0xf3 => true,
            _ => false,
        };
    }

    fn consume_gas(&mut self, gas: usize) {
        if self.gas >= gas {
            self.gas -= gas;
        } else {
            panic!("consume_gas: There is a shortage of gas.");
        }
    }

    /// トランザクションが終了するまでexecを繰り返す
    pub fn exec_transaction(&mut self, contract: &mut state::AccountState) {
        loop {
            if self.pc >= self.env.code.len() {
                break;
            }

            if self.exec(contract) {
                break;
            }
        }
    }

    pub fn disassemble(code: &str) {
        let mut env = Environment::new(
            Default::default(),
            Default::default(),
            1_000_000_000,
            100_000_000_000_000_000,
        );
        env.set_code(util::str_to_bytes(code));
        let mut vm = VM::new(env);
        let mut contract = state::AccountState::new(code.to_string());
        vm.exec_transaction(&mut contract);

        for mnemonic in vm.asm {
            println!("{}", mnemonic);
        }
    }

    fn push_asm(&mut self, mnemonic: &str) {
        self.asm.push(mnemonic.to_string());
    }
}

/// 0x00: 算術命令
impl VM {
    /// 0x00: 何もしない
    fn op_stop(&mut self) {
        self.push_asm("STOP");
    }

    /// 0x01: operand1(スタック1番目) + operand2(スタック2番目)
    fn op_add(&mut self) {
        self.consume_gas(3);
        self.push_asm("ADD");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 + operand2;
        self.push(result);
    }

    /// 0x02: operand1(スタック1番目) * operand2(スタック2番目)
    fn op_mul(&mut self) {
        self.consume_gas(5);
        self.push_asm("MUL");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 * operand2;
        self.push(result);
    }

    /// 0x03: operand1(スタック1番目) - operand2(スタック2番目)
    fn op_sub(&mut self) {
        self.consume_gas(3);
        self.push_asm("SUB");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 - operand2;
        self.push(result);
    }

    /// 0x04: operand1(スタック1番目) // operand2(スタック2番目)
    fn op_div(&mut self) {
        self.consume_gas(5);
        self.push_asm("DIV");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 / operand2;
        self.push(result);
    }

    fn op_sdiv(&mut self) {
        self.push_asm("SDIV");
        not_implement_panic();
    }

    fn op_mod(&mut self) {
        self.push_asm("MOD");
        not_implement_panic();
    }

    fn op_smod(&mut self) {
        self.push_asm("SMOD");
        not_implement_panic();
    }

    fn op_addmod(&mut self) {
        self.push_asm("ADDMOD");
        not_implement_panic();
    }

    fn op_mulmod(&mut self) {
        self.push_asm("MULMOD");
        not_implement_panic();
    }

    /// 0x0a: operand1(スタック1番目) ** operand2(スタック2番目)
    fn op_exp(&mut self) {
        self.consume_gas(10);
        self.push_asm("EXP");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1.pow(operand2);
        self.push(result);
    }

    /// 0x0b:
    fn op_sig_next_end(&mut self) {
        self.push_asm("SIGNEXTEND");
        not_implement_panic();
    }
}

/// 0x10: 条件、ビット演算
impl VM {
    /// 0x10: operand1(スタック1番目) < operand2(スタック2番目)
    fn op_lt(&mut self) {
        self.consume_gas(3);
        self.push_asm("LT");
        let operand1 = self.pop();
        let operand2 = self.pop();
        if operand1 < operand2 {
            self.push(U256::from(1));
        } else {
            self.push(U256::from(0));
        }
    }

    /// 0x11: operand1(スタック1番目) > operand2(スタック2番目)
    fn op_gt(&mut self) {
        self.consume_gas(3);
        self.push_asm("GT");
        let operand1 = self.pop();
        let operand2 = self.pop();
        if operand1 > operand2 {
            self.push(U256::from(1));
        } else {
            self.push(U256::from(0));
        }
    }

    fn op_slt(&mut self) {
        self.push_asm("SLT");
        not_implement_panic();
    }

    fn op_sgt(&mut self) {
        self.push_asm("SGT");
        not_implement_panic();
    }

    /// 0x14: operand1(スタック1番目) == operand2(スタック2番目)
    fn op_eq(&mut self) {
        self.consume_gas(3);
        self.push_asm("EQ");
        let operand1 = self.pop();
        let operand2 = self.pop();
        if operand1 == operand2 {
            self.push(U256::from(1));
        } else {
            self.push(U256::from(0));
        }
    }

    /// 0x15: operand1(スタック1番目) == 0
    fn op_is_zero(&mut self) {
        self.consume_gas(3);
        self.push_asm("ISZERO");
        let operand1 = self.pop();
        if operand1 == U256::from(0) {
            self.push(U256::from(1));
        } else {
            self.push(U256::from(0));
        }
    }

    /// operand1(スタック1番目) & operand2(スタック2番目)
    fn op_and(&mut self) {
        self.consume_gas(3);
        self.push_asm("AND");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 & operand2;
        self.push(result);
    }

    /// operand1(スタック1番目) | operand2(スタック2番目)
    fn op_or(&mut self) {
        self.consume_gas(3);
        self.push_asm("OR");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 | operand2;
        self.push(result);
    }

    /// operand1(スタック1番目) ^ operand2(スタック2番目)
    fn op_xor(&mut self) {
        self.consume_gas(3);
        self.push_asm("XOR");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let result = operand1 ^ operand2;
        self.push(result);
    }

    /// not operand1(スタック1番目)
    fn op_not(&mut self) {
        self.consume_gas(3);
        self.push_asm("NOT");
        let operand1 = self.pop();
        let result = !operand1;
        self.push(result);
    }

    /// 0x1a: operand2(スタック2番目)のoperand1バイト目を取る
    fn op_byte(&mut self) {
        // y = (operand2 >> (248 - operand1 * 8)) & 0xFF
        self.consume_gas(3);
        self.push_asm("BYTE");
        let operand1 = self.pop();
        let operand2 = self.pop();
        let mask = U256::from(0xff);
        let index = 248 - (operand1.as_u32() as usize) * 8;
        let result = (operand2 >> index) & mask;
        self.push(result);
    }

    fn op_shl(&mut self) {
        self.push_asm("SHL");
        not_implement_panic();
    }

    fn op_shr(&mut self) {
        self.push_asm("SHR");
        not_implement_panic();
    }

    fn op_sar(&mut self) {
        self.push_asm("SAR");
        not_implement_panic();
    }
}

/// 0x20: 暗号操作
impl VM {
    fn op_sha3(&mut self) {
        self.push_asm("SHA3");
        not_implement_panic();
    }
}

/// 0x30: 実行環境に関する操作 その1
impl VM {
    /// 0x30: address of the executing contract
    fn op_address(&mut self) {
        self.consume_gas(2);
        self.push_asm("ADDRESS");
        let address = util::h160_to_u256(&self.env.code_owner);
        self.push(address);
    }

    /// 0x31: Get balance of the given account.
    fn op_balance(&mut self) {
        self.consume_gas(400);
        self.push_asm("BALANCE");
        let address = util::u256_to_h160(&self.pop());
        // TODO: balanceを取得できるようにVMの状態を修正する
        not_implement_panic();
    }

    fn op_origin(&mut self) {
        self.push_asm("ORIGIN");
        not_implement_panic();
    }

    fn op_caller(&mut self) {
        self.consume_gas(2);
        self.push_asm("CALLER");
        self.push(util::h160_to_u256(&self.env.sender));
    }

    fn op_callvalue(&mut self) {
        self.push_asm("CALLVALUE");
        not_implement_panic();
    }

    /// 0x35: スタックからpopした値をstartとしてinputのstartの位置からstart+32の位置までの32byteのデータをstackにpush
    fn op_calldataload(&mut self) {
        self.consume_gas(3);
        self.push_asm("CALLDATALOAD");
        let start = self.pop().as_u32() as usize;
        let bytes: [u8; 32] = util::slice_to_array(&self.env.input[start..]);
        self.push(bytes.into());
    }

    /// 0x36: inputに格納されたデータサイズをstackにpush
    fn op_calldatasize(&mut self) {
        self.consume_gas(2);
        self.push_asm("CALLDATASIZE");
        let size = self.env.input.len();
        self.push(size.into());
    }

    /// 0x37:
    fn op_calldatacopy(&mut self) {
        self.push_asm("CALLDATACOPY");
        not_implement_panic();
    }

    /// 0x38:
    fn op_codesize(&mut self) {
        self.push_asm("CODESIZE");
        not_implement_panic();
    }

    /// 0x39: コントラクトにデプロイされたコードをコピーする
    fn op_codecopy(&mut self) {
        self.consume_gas(9); // ???
        self.push_asm("CODECOPY");
        let dest_offset = self.pop().as_u32() as usize;
        let offset = self.pop().as_u32() as usize;
        let length = self.pop().as_u32() as usize;

        for i in 0..length {
            let b = self.env.code[offset + i];
            self.memory.insert(dest_offset + i, b);
        }
    }

    /// 0x3a:
    fn op_gasprice(&mut self) {
        self.push_asm("GASPRICE");
        not_implement_panic();
    }

    /// 0x3b:
    fn op_extcodesize(&mut self) {
        self.push_asm("EXTCODESIZE");
        not_implement_panic();
    }

    /// 0x3c:
    fn op_extcodecopy(&mut self) {
        self.push_asm("EXTCODECOPY");
        not_implement_panic();
    }

    /// 0x3d:
    fn op_returndatasize(&mut self) {
        self.push_asm("RETURNDATASIZE");
        not_implement_panic();
    }

    /// 0x3e:
    fn op_returndatacopy(&mut self) {
        self.push_asm("RETURNDATACOPY");
        not_implement_panic();
    }

    /// 0x3f:
    fn op_extcodehash(&mut self) {
        self.push_asm("EXTCODEHASH");
        not_implement_panic();
    }
}

/// 0x40: 実行環境に関する操作 その2
impl VM {
    /// 0x40:
    fn op_blockhash(&mut self) {
        self.push_asm("BLOCKHASH");
        not_implement_panic();
    }

    /// 0x41:
    fn op_coinbase(&mut self) {
        self.push_asm("COINBASE");
        not_implement_panic();
    }

    /// 0x42:
    fn op_timestamp(&mut self) {
        self.push_asm("TIMESTAMP");
        not_implement_panic();
    }

    /// 0x43:
    fn op_number(&mut self) {
        self.push_asm("NUMBER");
        not_implement_panic();
    }

    /// 0x44:
    fn op_difficulty(&mut self) {
        self.push_asm("DIFFICULTY");
        not_implement_panic();
    }

    /// 0x45:
    fn op_gaslimit(&mut self) {
        self.push_asm("GASLIMIT");
        not_implement_panic();
    }
}

/// 0x50: EVM内のステート操作
impl VM {
    /// 0x50:
    fn op_pop(&mut self) {
        self.push_asm("POP");
        not_implement_panic();
    }

    /// 0x51: スタックからpopしたstartを先頭アドレスしてstart+32までの32byteの値をメモリからロード
    fn op_mload(&mut self) {
        self.consume_gas(3);
        self.push_asm("MLOAD");
        let start = self.pop().as_u32() as usize;
        let mut bytes: [u8; 32] = [0; 32];
        for i in 0..32 {
            let b = self.memory[start + i];
            bytes[i] = b;
        }
        self.push(bytes.into());
    }

    /// 0x52: スタックからstart, valueをpopし、startを先頭アドレスしてstart+32までの32byteのメモリ領域にvalueを格納する
    fn op_mstore(&mut self) {
        self.consume_gas(6);
        self.push_asm("MSTORE");
        let address = self.pop().as_u32() as usize;
        let value = self.pop();
        let bytes: [u8; 32] = value.into();
        for (i, b) in bytes.iter().enumerate() {
            self.memory.insert(address + i, *b);
        }
    }

    /// 0x53:
    fn op_mstore8(&mut self) {
        self.push_asm("MSTORE8");
        not_implement_panic();
    }

    /// 0x54: スタックからpopした値をkeyとしてstorageから対応する値をロード
    fn op_sload(&mut self, contract: &mut state::AccountState) {
        self.consume_gas(200);
        self.push_asm("SLOAD");
        let key = self.pop();
        let value = contract.get_storage(&key);
        self.push(*value);
    }

    /// 0x55: storageに書き込みを行う storage[operand1(スタック1番目)] = operand2(スタック2番目)
    fn op_sstore(&mut self, contract: &mut state::AccountState) {
        let key = self.pop();
        let value = self.pop();

        // ストレージへの書き込みは書き込み先と書き込むデータによってgasが変動する
        if (key == U256::from(0)) && (value != U256::from(0)) {
            self.consume_gas(20000);
        } else {
            self.consume_gas(5000);
        }
        self.push_asm("SSTORE");

        contract.set_storage(key, value);
    }

    /// 0x56: スタックからdestinationをpopしてジャンプ
    fn op_jump(&mut self) {
        self.consume_gas(8);
        self.push_asm("JUMP");
        let destination = self.pop().as_u32() as usize;
        // ジャンプ先のアドレスのオペコードはJUMPDESTでなければならない
        if self.env.code[destination] != 0x5b {
            panic!("op_jump: destination must be JUMPDEST");
        }

        self.pc = destination + 1; // TODO: +1が必要か調査する
    }

    /// 0x57: スタックからdestination, conditionをpop<br/>
    /// conditionが0以外ならdestinationにジャンプ
    fn op_jumpi(&mut self) {
        self.consume_gas(10);
        self.push_asm("JUMPI");
        let destination = self.pop().as_u32() as usize;
        let condition = self.pop().as_u32() as usize;
        // ジャンプ先のアドレスのオペコードはJUMPDESTでなければならない
        if self.env.code[destination] != 0x5b {
            panic!("op_jumpi: destination must be JUMPDEST");
        }

        // conditionか0ならジャンプする
        if condition != 0 {
            self.pc = destination + 1; // TODO: +1が必要か調査する
        }
    }

    /// 0x58:
    fn op_pc(&mut self) {
        self.push_asm("PC");
        not_implement_panic();
    }

    /// 0x59
    fn op_msize(&mut self) {
        self.push_asm("MSIZE");
        not_implement_panic();
    }

    /// 0x5a:
    fn op_gas(&mut self) {
        self.push_asm("GAS");
        not_implement_panic();
    }

    /// 0x5b: 動的ジャンプを行う際にスタックからpopした値が示すアドレスにジャンプするが、そのアドレスではこのop_jumpdestがオペコードでなければならない<br/>
    /// このオペコードはそのマーカーとなるだけで単体では意味を持たない
    fn op_jumpdest(&mut self) {
        self.consume_gas(1);
        self.push_asm("JUMPDEST");
    }
}

/// 0x60-0x7f: PUSH命令
impl VM {
    /// lengthバイトpushする
    fn op_push(&mut self, length: usize) {
        let mut operand = [0; 32];
        let mut operand_str = "".to_string();
        for i in 0..length {
            operand[32 - length + i] = self.env.code[self.pc];
            operand_str += &hex::encode(vec![self.env.code[self.pc]]);
            self.pc += 1;
        }
        self.consume_gas(3);
        let asm = "PUSH".to_string() + " " + &operand_str;
        self.push_asm(&asm);
        self.push(operand.into());
    }
}

/// 0x80: DUP命令
impl VM {
    /// スタックの先頭をスタックのindex+1番目にコピーする
    fn op_dup(&mut self, index: usize) {
        self.consume_gas(3);
        let operand = self.stack[self.sp - 1];
        self.push_asm("DUP");
        if self.sp > 1 {
            self.stack[self.sp - index - 1] = operand;
        } else {
            self.push(operand);
        }
    }
}

/// 0x90: SWAP命令
impl VM {
    /// スタックの先頭をスタックのindex+1番目と交換する
    fn op_swap(&mut self, index: usize) {
        self.consume_gas(3);
        self.push_asm("SWAP");
        let operand1 = self.stack[self.sp - 1];
        let operand2 = self.stack[self.sp - index - 1];
        self.stack[self.sp - 1] = operand2;
        self.stack[self.sp - index - 1] = operand1;
    }
}

/// 0xa0: ログ
impl VM {
    /// 0xa0:
    fn op_log0(&mut self) {
        self.push_asm("LOG0");
        not_implement_panic();
    }

    /// 0xa1:
    fn op_log1(&mut self) {
        self.push_asm("LOG1");
        not_implement_panic();
    }

    /// 0xa2:
    fn op_log2(&mut self) {
        self.push_asm("LOG2");
        not_implement_panic();
    }

    /// 0xa3:
    fn op_log3(&mut self) {
        self.push_asm("LOG3");
        not_implement_panic();
    }

    /// 0xa4:
    fn op_log4(&mut self) {
        self.push_asm("LOG4");
        not_implement_panic();
    }
}

/// 0xf0:
impl VM {
    /// 0xf0:
    fn op_create(&mut self) {
        self.push_asm("CREATE");
        not_implement_panic();
    }

    /// 0xf1:
    fn op_call(&mut self) {
        self.push_asm("CALL");
        not_implement_panic();
    }

    /// 0xf2:
    fn op_callcode(&mut self) {
        self.push_asm("CALLCODE");
        not_implement_panic();
    }

    /// 0xf3: スタックのoffsetからlength分のバイトデータを返り値として返す<br/>
    /// この命令を実行するとトランザクションは終了する？
    fn op_return(&mut self) {
        self.push_asm("RETURN");
        let offset = self.pop().as_u32() as usize;
        let length = self.pop().as_u32() as usize;

        let return_value = &self.memory[offset..offset + length];
        self.returns = Vec::from(return_value);
    }

    /// 0xf4:
    fn op_delegatecall(&mut self) {
        self.push_asm("DELEGATECALL");
        not_implement_panic();
    }

    /// 0xf5:
    fn op_create2(&mut self) {
        self.push_asm("CREATE2");
        not_implement_panic();
    }

    /// 0xfa:
    fn op_staticcall(&mut self) {
        self.push_asm("STATICCALL");
        not_implement_panic();
    }

    /// 0xfd:
    fn op_revert(&mut self) {
        self.push_asm("REVERT");
        not_implement_panic();
    }

    /// 0xff:
    fn op_selfdestruct(&mut self) {
        self.push_asm("SELFDESTRUCT");
        not_implement_panic();
    }
}

#[test]
fn test_new() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600401"));
    let vm = VM::new(env);
    assert_eq!(vm.env.code, vec![0x60, 0x05, 0x60, 0x04, 0x01]);
    assert_eq!(vm.pc, 0);
    assert_eq!(vm.gas, 10000000000);
    assert_eq!(vm.sp, 0);
    assert_eq!(vm.stack, Vec::default());
}

#[test]
fn test_push1() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 2);
    assert_eq!(vm.gas, 9999999997);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![5.into()]);
}

#[test]
fn test_add() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600401"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999991);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![9.into()]);
}

#[test]
fn test_sub() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6004600503"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999991);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![1.into()]);
}

#[test]
fn test_mul() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6003600602"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999989);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![18.into()]);
}

#[test]
fn test_div() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6003600604"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999989);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![2.into()]);
}

#[test]
fn test_exp() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("600360020a"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999984);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![8.into()]);
}

#[test]
fn test_mstore() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600401600052"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 8);
    assert_eq!(vm.gas, 9999999982);
    assert_eq!(vm.sp, 0);
    assert_eq!(vm.memory[0x1f], 0x09);
}

#[test]
fn test_mload() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600401600052600051"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 11);
    assert_eq!(vm.gas, 9999999976);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![0x09.into()]);
}

#[test]
fn test_add2() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("61010161010201"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 7);
    assert_eq!(vm.gas, 9999999991);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![0x0203.into()]);
}

#[test]
fn test_calldataload() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("60003560203501"));
    env.set_input(util::str_to_bytes("00000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000004"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 7);
    assert_eq!(vm.gas, 9999999985);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![0x09.into()]);
}

#[test]
fn test_calldatasize() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("36"));
    env.set_input(util::str_to_bytes(
        "0000000000000000000000000000000000000000000000000000000000000005",
    ));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 1);
    assert_eq!(vm.gas, 9999999998);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.stack, vec![32.into()]);
}

#[test]
fn test_jumpi() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes(
        "6000356000525b600160005103600052600051600657",
    ));
    env.set_input(util::str_to_bytes(
        "0000000000000000000000000000000000000000000000000000000000000005",
    ));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    for _ in 0..14 {
        vm.exec(&mut contract);
    }
    assert_eq!(vm.pc, 21); // jumpi
    vm.exec(&mut contract); // ここでジャンプ
    assert_eq!(vm.pc, 7);
}

#[test]
fn test_dup1() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600480"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999991);
    assert_eq!(vm.sp, 2);
    assert_eq!(vm.stack, vec![0x04.into(), 0x04.into()]);
}

#[test]
fn test_swap1() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6005600490"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 5);
    assert_eq!(vm.gas, 9999999991);
    assert_eq!(vm.sp, 2);
    assert_eq!(vm.stack, vec![0x04.into(), 0x05.into()]);
}

#[test]
fn test_loop() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("6000355b6001900380600357"));
    env.set_input(util::str_to_bytes(
        "0000000000000000000000000000000000000000000000000000000000000005",
    ));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    for _ in 0..8 {
        vm.exec(&mut contract);
    }
    assert_eq!(vm.pc, 11); // jumpi
    vm.exec(&mut contract); // ここでジャンプ
    assert_eq!(vm.pc, 4);
    for _ in 0..5 {
        vm.exec(&mut contract);
    }
    assert_eq!(vm.pc, 11); // jumpi
    vm.exec(&mut contract); // ここでジャンプ
    assert_eq!(vm.pc, 4);
}

/// 短いインプットに依存するループのテスト
///
/// ```
/// CALLDATASIZE 10000000000 -> 9999999998  36
/// PUSH1  => 20 9999999998 -> 9999999995   6020
/// SUB          9999999995 -> 9999999992   03
/// PUSH2  => 0100 9999999992 -> 9999999989 610100
/// EXP             9999999989 -> 9999999979    0a
/// PUSH1  => 00  9999999979 -> 9999999976  6000
/// CALLDATALOAD 9999999976 -> 9999999973   35
/// DIV         9999999973 -> 9999999968    04
/// JUMPDEST    9999999968 -> 9999999967    5b
/// PUSH1  => 01    9999999967 -> 9999999964    6001
/// SWAP1           9999999964 -> 9999999961    90
/// SUB             9999999961 -> 9999999958    03
/// DUP1            9999999958 -> 9999999955    80
/// PUSH1  => 0c       9999999955 -> 9999999952
/// JUMPI           9999999952 -> 9999999942
/// ```
#[test]
fn test_loop2() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes(
        "366020036101000a600035045b6001900380600c57",
    ));
    env.set_input(util::str_to_bytes("01"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 21);
    assert_eq!(vm.gas, 9999999942);
}

#[test]
fn test_deploy() {
    let mut env = Environment::new(
        Default::default(),
        Default::default(),
        10_000_000,
        100_000_000_000_000_000,
    );
    env.set_code(util::str_to_bytes("600580600b6000396000f36005600401"));
    let mut vm = VM::new(env);
    let mut contract = state::AccountState::new("".to_string());
    vm.exec_transaction(&mut contract);
    assert_eq!(vm.pc, 11);
    assert_eq!(vm.gas, 9999999976);
    assert_eq!(vm.sp, 0);
}
