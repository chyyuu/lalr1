use crate::{Act, ConflictKind, Conflict, TableEntry, Table, Lr1Fsm, Lr1Node, Lr1Item, Link};
use std::{cmp::Ordering, borrow::Borrow};
use grammar_config::{Assoc, AbstractGrammar, AbstractGrammarExt};
use smallvec::{smallvec, SmallVec};
use hashbrown::HashMap;

pub fn mk_table<'a, L: Borrow<Link>>(lr1: &'a Lr1Fsm<'a, L>, g: &'a impl AbstractGrammar<'a>) -> Table<'a> {
  let mut table = Vec::with_capacity(lr1.len());
  let eof = g.eof();
  let start_id = (g.start().1).1;
  let token_num = g.token_num();
  for Lr1Node { closure, link } in lr1 {
    let link = link.borrow();
    let (mut act, mut goto) = (HashMap::new(), HashMap::new());
    for (&k, &v) in link {
      if k < g.nt_num() {
        goto.insert(k, v);
      } else {
        act.insert(k, smallvec![Act::Shift(v)]);
      }
    }
    for Lr1Item {lr0, lookahead  } in closure {
      if lr0.dot == lr0.prod.len() as u32 {
        if lookahead.test(eof as usize) && lr0.prod_id == start_id {
          act.insert(eof, smallvec![Act::Acc]);
        } else {
          for i in 0..token_num {
            if lookahead.test(i as usize) {
              // maybe conflict here
              act.entry(i).or_insert_with(SmallVec::new).push(Act::Reduce(lr0.prod_id));
            }
          }
        }
      }
    }
//    for (item, Lr1Item { lookahead, .. }) in closure.iter().zip(result[i].iter()) {
//      if item.dot == item.prod.len() as u32 {
//        if lookahead.test(eof as usize) && item.prod_id == start_id {
//          act.insert(eof, smallvec![Act::Acc]);
//        } else {
//          for i in 0..token_num {
//            if lookahead.test(i as usize) {
//              // maybe conflict here
//              act.entry(i).or_insert_with(SmallVec::new).push(Act::Reduce(item.prod_id));
//            }
//          }
//        }
//      }
//    }
    table.push(TableEntry { closure, act, goto });
  }
  table
}

// Reference: https://docs.oracle.com/cd/E19504-01/802-5880/6i9k05dh3/index.html
// A precedence and associativity is associated with each grammar rule.
// It is the precedence and associativity of the **final token or literal** in the body of the rule.
// If the %prec construction is used, it overrides this default value.
// Some grammar rules may have no precedence and associativity associated with them.
//
// When there is a reduce-reduce or shift-reduce conflict, and **either** the input symbol or the grammar rule has no precedence and associativity,
// then the two default disambiguating rules given in the preceding section are used, and the **conflicts are reported**.
//   In a shift-reduce conflict, the default is to shift.
//   In a reduce-reduce conflict, the default is to reduce by the earlier grammar rule (in the yacc specification).
// If there is a shift-reduce conflict and both the grammar rule and the input character have precedence and associativity associated with them,
// then the conflict is resolved in favor of the action -- shift or reduce -- associated with the higher precedence.
// If precedences are equal, then associativity is used.
// Left associative implies reduce; right associative implies shift; nonassociating implies error.

// `solve` will modify t in these ways:
// for conflicts solved based on precedence and/or associativity, other choices are removed
// for conflicts solved based on location or "shift better than reduced", other choices are NOT removed
// in both cases, the selected choice is placed at [0]
pub fn solve<'a>(t: &mut Table<'a>, g: &'a impl AbstractGrammarExt<'a>) -> Vec<Conflict> {
  use Act::{Reduce, Shift};
  let mut reports = Vec::new();
  for (idx, t) in t.iter_mut().enumerate() {
    for (&ch, acts) in &mut t.act {
      match acts.as_slice() {
        [] | [_] => {}
        &[a0, a1] => match (a0, a1) {
          (Reduce(r1), Reduce(r2)) =>
            *acts = match (g.prod_pri(r1), g.prod_pri(r2)) {
              (Some(p1), Some(p2)) if p1 != p2 => smallvec![Reduce(if p1 < p2 { r2 } else { r1 })],
              _ => {
                reports.push(Conflict { kind: ConflictKind::RR { r1, r2 }, state: idx as u32, ch });
                smallvec![Reduce(r1.min(r2)), Reduce(r1.max(r2))]
              }
            },
          (Reduce(r), Shift(s)) | (Shift(s), Reduce(r)) =>
            *acts = match (g.prod_pri(r), g.term_pri_assoc(ch)) {
              (Some(pp), Some((cp, ca))) => match pp.cmp(&cp) {
                Ordering::Less => smallvec![Shift(s)],
                Ordering::Greater => smallvec![Reduce(r)],
                Ordering::Equal => match ca {
                  Assoc::Left => smallvec![Reduce(r)],
                  Assoc::Right => smallvec![Shift(s)],
                  Assoc::NoAssoc => smallvec![],
                }
              },
              _ => {
                reports.push(Conflict { kind: ConflictKind::SR { s, r }, state: idx as u32, ch });
                smallvec![Shift(s), Reduce(r)]
              }
            },
          _ => unreachable!("There should be a bug in lr."),
        }
        _ => reports.push(Conflict { kind: ConflictKind::Many(acts.clone()), state: idx as u32, ch }),
      }
    }
  }
  reports
}