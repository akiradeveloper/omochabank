use crate::*;
use std::collections::HashMap;

type DiffAmount = fixed::types::I114F14;

// Deposite and Withdrawal are different only by the sign.
#[derive(Debug)]
struct ChangeDeposit {
    add_available: DiffAmount,
    add_total: DiffAmount,
}
impl ChangeDeposit {
    fn mk_dispute(&self) -> Dispute {
        Dispute {
            add_available: -self.add_available,
            add_held: self.add_available,
        }
    }
}
// The type claims what balances will be changed.
// This is good for readability.
#[derive(Debug)]
struct Dispute {
    add_available: DiffAmount,
    add_held: DiffAmount,
}
#[derive(Debug)]
struct Resolve {
    add_available: DiffAmount,
    add_held: DiffAmount,
}
#[derive(Debug)]
struct Chargeback {
    add_held: DiffAmount,
    add_total: DiffAmount,
}
impl Dispute {
    // Here consumes so compiler can ensure the instance is
    // removed from the hash table.
    fn into_resolve(self) -> Resolve {
        Resolve {
            // These calculation looks complicated but
            // can be fixed by simple tests.
            add_available: -self.add_available,
            add_held: -self.add_held,
        }
    }
    fn into_chargeback(self) -> Chargeback {
        Chargeback {
            add_held: -self.add_held,
            add_total: self.add_available,
        }
    }
}
pub struct TxCompute {
    add_available: DiffAmount,
    add_held: DiffAmount,
    add_total: DiffAmount,

    changes: HashMap<TxId, ChangeDeposit>,
    disputes: HashMap<TxId, Dispute>,

    locked: bool,
}
impl TxCompute {
    pub fn new() -> Self {
        Self {
            add_available: 0.into(),
            add_held: 0.into(),
            add_total: 0.into(),

            changes: HashMap::new(),
            disputes: HashMap::new(),

            locked: false,
        }
    }
    pub fn output(&self, client_id: ClientId) -> String {
        format!(
            "{},{},{},{},{}",
            client_id, self.add_available, self.add_held, self.add_total, self.locked
        )
    }
    pub fn execute_command(&mut self, command: TxCommand) {
        if self.locked {
            return;
        }
        match command {
            TxCommand::Deposit { tx, amount } => {
                // eff = effect
                // I name this because it is an effect to the balance.
                let eff = ChangeDeposit {
                    add_available: amount,
                    add_total: amount,
                };
                self.add_available += eff.add_available;
                self.add_total += eff.add_total;

                self.changes.insert(tx, eff);
            }
            TxCommand::Withdrawal { tx, amount } => {
                let eff = ChangeDeposit {
                    add_available: -amount,
                    add_total: -amount,
                };
                // No debt!
                if self.add_available + eff.add_available < DiffAmount::ZERO {
                    return;
                }
                self.add_available += eff.add_available;
                self.add_total += eff.add_total;

                self.changes.insert(tx, eff);
            }
            TxCommand::Dispute { tx } => {
                // On-going dispute exists, skip
                if self.disputes.contains_key(&tx) {
                    return;
                }
                // The target tx does not exist, skip
                if !self.changes.contains_key(&tx) {
                    return;
                }
                let change = self.changes.get(&tx).unwrap();
                let eff = change.mk_dispute();
                // Try dispute. Should deny tx that falls in debt.
                // I do so because dispute is a request to cancel the transaction so
                // should be denied as withdrawal.
                if self.add_available + eff.add_available < DiffAmount::ZERO {
                    return;
                }

                self.add_available += eff.add_available;
                self.add_held += eff.add_held;

                self.disputes.insert(tx, eff);
            }
            TxCommand::Resolve { tx } => {
                if !self.disputes.contains_key(&tx) {
                    return;
                }
                let dispute = self.disputes.remove(&tx).unwrap();
                let eff = dispute.into_resolve();
                self.add_available += eff.add_available;
                self.add_held += eff.add_held;
            }
            TxCommand::Chargeback { tx } => {
                if !self.disputes.contains_key(&tx) {
                    return;
                }
                let dispute = self.disputes.remove(&tx).unwrap();
                let eff = dispute.into_chargeback();
                self.add_held += eff.add_held;
                self.add_total += eff.add_total;

                self.locked = true;
            }
        }
        // We always check the invariant.
        assert_eq!(self.add_total, self.add_available + self.add_held);
    }
}

#[cfg(test)]
mod tests {
    use crate::Amount;
    use crate::TxCommand::*;

    use super::DiffAmount;
    fn amount(x: f32) -> Amount {
        Amount::from_num(x)
    }
    fn to_f32(x: DiffAmount) -> f32 {
        use az::Cast;
        x.cast()
    }
    fn run(xs: impl IntoIterator<Item = crate::TxCommand>) -> (f32, f32, f32) {
        let mut st = super::TxCompute::new();
        for x in xs {
            st.execute_command(x);
        }
        let out = (
            to_f32(st.add_available),
            to_f32(st.add_held),
            to_f32(st.add_total),
        );
        out
    }
    #[test]
    fn test_normal() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(2.0)
                },
            ]),
            (1., 0., 1.)
        );
    }
    #[test]
    fn test_mal_withdrawal() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(4.0)
                },
            ]),
            (3., 0., 3.)
        );
    }
    #[test]
    fn test_mal_dispute() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(2.0)
                },
                // This dispute should be denied.
                Dispute { tx: 1 },
            ]),
            (1., 0., 1.)
        );
    }
    #[test]
    fn test_normal_dispute() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(2.0)
                },
                Dispute { tx: 2 },
            ]),
            (3., -2., 1.)
        );
    }
    #[test]
    fn test_resolve() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(2.0)
                },
                Dispute { tx: 2 },
                Resolve { tx: 2 },
            ]),
            (1., 0., 1.)
        );
    }
    #[test]
    fn test_chargeback() {
        assert_eq!(
            run([
                Deposit {
                    tx: 1,
                    amount: amount(3.0)
                },
                Withdrawal {
                    tx: 2,
                    amount: amount(2.0)
                },
                Dispute { tx: 2 },
                Chargeback { tx: 2 },
            ]),
            (3., 0., 3.)
        );
    }
}
