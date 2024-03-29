use crate::*;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(serde::Deserialize)]
struct Row(String, ClientId, TxId, Option<fixed::types::I114F14>);

pub fn parse(path: impl AsRef<Path>) -> std::io::Result<impl Iterator<Item = Option<Tx>>> {
    let f = File::open(path.as_ref())?;
    let rdr = BufReader::new(f);
    let rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(rdr);
    let it = rdr.into_deserialize::<Row>().map(|x| match x {
        Ok(Row(ty, cli, tx, am)) => match (ty.as_ref(), cli, tx, am) {
            ("deposit", cli, tx, Some(amount)) => Some(Tx {
                client_id: cli,
                command: TxCommand::Deposit { tx, amount },
            }),
            ("withdrawal", cli, tx, Some(amount)) => Some(Tx {
                client_id: cli,
                command: TxCommand::Withdrawal { tx, amount },
            }),
            ("dispute", cli, tx, None) => Some(Tx {
                client_id: cli,
                command: TxCommand::Dispute { tx },
            }),
            ("resolve", cli, tx, None) => Some(Tx {
                client_id: cli,
                command: TxCommand::Resolve { tx },
            }),
            ("chargeback", cli, tx, None) => Some(Tx {
                client_id: cli,
                command: TxCommand::Chargeback { tx },
            }),
            _ => None,
        },
        // If the line is invalid, let's return none.
        Err(_) => None,
    });
    Ok(it)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_huge_number() {
        // 2^110 + 0.1234
        let x = "1298074214633706907132624082305024.1234";
        dbg!(Amount::from_str(x).unwrap());
    }
    #[test]
    fn test_parse() {
        let mut rows = vec![];
        for row in parse("sample.csv").unwrap() {
            assert!(row.is_some());
            rows.push(row.unwrap());
        }
        dbg!(&rows);
        assert_eq!(rows.len(), 10);
    }
}
