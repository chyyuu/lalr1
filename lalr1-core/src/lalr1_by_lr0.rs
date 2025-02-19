// "Compilers: Principles, Techniques and Tools" Algorithm 4.63

use crate::{lr1::Lr1Ctx, Lr1Item, Lr0Fsm, Lr0Node, Lr1Node, Lr1Fsm, Link};
use grammar_config::AbstractGrammar;
use hashbrown::HashMap;
use bitset::BitSet;

pub fn work<'a, 'b>(lr0: &'b Lr0Fsm<'a>, g: &'a impl AbstractGrammar<'a>) -> Lr1Fsm<'a, &'b Link> {
  let mut ctx = Lr1Ctx::new(g);
  let mut lookahead = lr0.iter()
    .map(|Lr0Node { closure, .. }| vec![BitSet::new(ctx.0.token_num); closure.len()]).collect::<Vec<_>>();
  let mut prop = Vec::new();
  let start_prod = (g.start().1).0.as_ref();

  for (i, item) in lr0[0].closure.iter().enumerate() {
    if item.prod == start_prod {
      lookahead[0][i].set(g.eof() as usize);
      break;
    }
  }

  let special_term = g.err() as usize;
  for (i, Lr0Node { closure: state, link }) in lr0.iter().enumerate() {
    for (item_id, &item) in state.iter().enumerate() {
      // only consider lr0 core item
      if item.prod == start_prod || item.dot != 0 {
        let cl = ctx.closure({
                               let mut lookahead = BitSet::new(ctx.0.token_num);
                               lookahead.set(special_term);
                               let mut init = HashMap::new();
                               init.insert(item, lookahead);
                               init
                             }, g);
        let from = lookahead[i][item_id].as_ptr();
        for Lr1Item { lr0: cl_item, lookahead: cl_lookahead } in &cl {
          if cl_item.dot as usize >= cl_item.prod.len() {
            continue;
          }
          let goto_state = link[&cl_item.prod[cl_item.dot as usize]];
          let goto_item_id = cl_item.unique_id() + 1; // dot + 1
          let goto_item_idx = lr0[goto_state as usize].closure.iter().enumerate().find(|item| item.1.unique_id() == goto_item_id).unwrap().0;
          let goto_lookahead = &mut lookahead[goto_state as usize][goto_item_idx];
          goto_lookahead.or(&cl_lookahead);
          if cl_lookahead.test(special_term) {
            prop.push((from, goto_lookahead.as_mut_ptr()));
          }
        }
      }
    }
  }

  let mut changed = true;
  let len = lookahead[0][0].inner_len();
  while changed {
    changed = false;
    unsafe {
      for &(from, to) in &prop {
        changed |= BitSet::or_raw(to, from, len);
      }
    }
  }

  lookahead.iter_mut().for_each(|l| l.iter_mut().for_each(|l| l.clear(special_term)));

  lr0.iter().zip(lookahead.into_iter()).map(|(node, lookahead)| Lr1Node {
    closure: ctx.closure(node.closure.clone().into_iter().zip(lookahead.into_iter()).collect(), g),
    link: &node.link,
  }).collect()
}

//pub fn work<'a>(lr0: &'a Lr0Fsm<'a>, g: &'a impl AbstractGrammarExt<'a>) -> Table<'a> {
//  let result = lalr1_by_lr0(lr0, g);

//}
