//! Seven deterministic cutscene atom archetypes.
#![allow(dead_code)]
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum CutsceneAtomKind{Arrival,Witness,Severance,Refusal,Revelation,Exchange,Vanishing}
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum AtomLens{Explore3D,Combat2D,TacticalLedger,RootRelation,Strategic4X,VoidLeak,VowlessBlank}
#[derive(Debug,Clone,Copy)] pub struct CutsceneAtom{pub kind:CutsceneAtomKind,pub lens:AtomLens,pub subject_hash:u64,pub target_hash:u64,pub duration_ticks:u32,pub suppress_ui:bool,pub record_hash:u64}
pub fn atom_key(kind:CutsceneAtomKind,lens:AtomLens,subject_hash:u64,target_hash:u64)->u64{let mut h=0xcbf29ce484222325u64;for v in[kind as u64,lens as u64,subject_hash,target_hash]{h^=v;h=h.wrapping_mul(0x100000001b3)}h}
pub fn make_atom(kind:CutsceneAtomKind,lens:AtomLens,subject_hash:u64,target_hash:u64,duration_ticks:u32)->CutsceneAtom{CutsceneAtom{kind,lens,subject_hash,target_hash,duration_ticks,suppress_ui:matches!(kind,CutsceneAtomKind::Refusal)||matches!(lens,AtomLens::VowlessBlank),record_hash:atom_key(kind,lens,subject_hash,target_hash)}}
