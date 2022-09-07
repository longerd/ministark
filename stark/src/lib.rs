#![feature(generic_const_exprs)]

use algebra::Felt;
use algebra::Multivariate;
use algebra::PrimeFelt;
use algebra::StarkFelt;
use brainfuck::InputTable;
use brainfuck::InstructionTable;
use brainfuck::MemoryTable;
use brainfuck::OutputTable;
use brainfuck::ProcessorTable;
use brainfuck::Table;
use protocol::ProofStream;
use std::cmp::max;
use std::marker::PhantomData;
use std::vec;

mod protocol;

pub struct StarkParams {
    /// power of 2 expansion factor
    expansion_factor: usize,
    /// security level of generated proofs
    security_level: usize,
    // TODO: fri params. folding factor, queries, etc.
}

impl StarkParams {
    pub fn new(expansion_factor: usize, security_level: usize) -> StarkParams {
        assert!(expansion_factor >= 4, "must be 4 or greater");
        assert!(expansion_factor.is_power_of_two(), "not a power of two");
        StarkParams {
            expansion_factor,
            security_level,
        }
    }

    pub fn num_randomizers(&self) -> usize {
        self.security_level
    }

    pub fn security_level(&self) -> usize {
        self.security_level
    }

    pub fn expansion_factor(&self) -> usize {
        self.expansion_factor
    }
}

pub struct BrainFuckStark<E> {
    params: StarkParams,
    processor_table: ProcessorTable<E>,
    memory_table: MemoryTable<E>,
    instruction_table: InstructionTable<E>,
    input_table: InputTable<E>,
    output_table: OutputTable<E>,
}

impl<E: PrimeFelt + StarkFelt> BrainFuckStark<E> {
    pub fn new(params: StarkParams) -> BrainFuckStark<E> {
        let num_randomizers = params.num_randomizers();
        BrainFuckStark {
            params,
            processor_table: ProcessorTable::new(num_randomizers),
            memory_table: MemoryTable::new(num_randomizers),
            instruction_table: InstructionTable::new(num_randomizers),
            input_table: InputTable::new(),
            output_table: OutputTable::new(),
        }
    }

    fn fri_codeword_length(&self) -> usize {
        assert!(!self.processor_table.is_empty(), "tables not populated");
        // TODO: could be a bug here... Instead of rounding up to the power of two it
        // should be the next power of two.
        let max_degree = self
            .processor_table
            .max_degree()
            .max(self.memory_table.max_degree())
            .max(self.instruction_table.max_degree())
            .max(self.input_table.max_degree())
            .max(self.output_table.max_degree());
        ceil_power_of_two(max_degree) * self.params.expansion_factor
    }

    pub fn prove<T: ProofStream<E>>(
        &mut self,
        processor_matrix: Vec<[E; ProcessorTable::<E>::BASE_WIDTH]>,
        memory_matrix: Vec<[E; MemoryTable::<E>::BASE_WIDTH]>,
        instruction_matrix: Vec<[E; InstructionTable::<E>::BASE_WIDTH]>,
        input_matrix: Vec<[E; InputTable::<E>::BASE_WIDTH]>,
        output_matrix: Vec<[E; OutputTable::<E>::BASE_WIDTH]>,
        proof_stream: &mut T,
    ) -> Vec<u8> {
        let padding_length = {
            let max_length = processor_matrix
                .len()
                .max(memory_matrix.len())
                .max(instruction_matrix.len())
                .max(input_matrix.len())
                .max(output_matrix.len());
            ceil_power_of_two(max_length)
        };

        let Self {
            processor_table,
            memory_table,
            instruction_table,
            input_table,
            output_table,
            ..
        } = self;

        processor_table.set_matrix(processor_matrix);
        memory_table.set_matrix(memory_matrix);
        instruction_table.set_matrix(instruction_matrix);
        input_table.set_matrix(input_matrix);
        output_table.set_matrix(output_matrix);

        // pad tables to height 2^k
        processor_table.pad(padding_length);
        memory_table.pad(padding_length);
        instruction_table.pad(padding_length);
        input_table.pad(padding_length);
        output_table.pad(padding_length);

        // let tables = vec![
        //     &processor_table as &dyn Table<E>,
        //     &memory_table as &dyn Table<E>,
        //     &instruction_table as &dyn Table<E>,
        //     &input_table as &dyn Table<E>,
        //     &output_table as &dyn Table<E>,
        // ];

        // let max_degree = tables.iter().map(|table|
        // table.max_degree()).max().unwrap(); let fri_domain_length =

        Vec::new()
    }
}

/// Rounds the input value up the the nearest power of two
fn ceil_power_of_two(value: usize) -> usize {
    if value.is_power_of_two() {
        value
    } else {
        value.next_power_of_two()
    }
}