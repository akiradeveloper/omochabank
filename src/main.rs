use std::collections::HashMap;

mod compute;
mod reader;

// Non-negative amount.
type Amount = rust_decimal::Decimal;
type ClientId = u16;
type TxId = u32;

#[derive(Debug)]
pub enum TxCommand {
    Deposit { tx: TxId, amount: Amount },
    Withdrawal { tx: TxId, amount: Amount },
    Dispute { tx: TxId },
    // Retract dispute.
    Resolve { tx: TxId },
    // Finalize dispute and lock.
    Chargeback { tx: TxId },
}

#[derive(Debug)]
pub struct Tx {
    client_id: ClientId,
    command: TxCommand,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).unwrap();
    let mut computes = HashMap::new();
    for tx in reader::parse(path).unwrap() {
        // If the file is in bad format, we can't trust the input.
        // Let's abort the entire transactions.
        let Tx { client_id, command } = tx.expect("invalid row");
        let compute = computes
            .entry(client_id)
            .or_insert(compute::TxCompute::new());
        compute.execute_command(command);
    }
    print!("client,available,held,total,locked");
    for (client_id, compute) in computes {
        print!("\n{}", compute.output(client_id))
    }
}
