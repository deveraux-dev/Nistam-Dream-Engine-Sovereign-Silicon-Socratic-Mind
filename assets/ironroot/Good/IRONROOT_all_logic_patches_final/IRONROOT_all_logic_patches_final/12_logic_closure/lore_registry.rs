//! Registries and world-first policy.
#![allow(dead_code)]
use crate::lore_core::*;
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct FirstLockId(pub u16);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct RelicId(pub u32);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct PuzzleScarId(pub u32);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum Era{Ancient,Golden,Decay,Void}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum FirstLockFinalAction{Kill,Spare,Craft,Bury,Refuse,SpeakName,StaySilent,LoseOnPurpose,DieInPlace,WalkAway,RingBell,DoNotRingBell}
#[derive(Debug,Clone,Copy)] pub struct FirstLockDef{pub id:FirstLockId,pub slug:&'static str,pub account:HiddenAccount,pub zone:ZoneHash,pub required_eras:&'static [Era],pub demand_a:MechanicalDemand,pub demand_b:MechanicalDemand,pub final_action:FirstLockFinalAction,pub world_first_relic:RelicId,pub echo_relic:RelicId,pub puzzle_scar:PuzzleScarId,pub public_ledger_line:&'static str}
#[derive(Debug,Clone,Copy)] pub struct WorldFirstClaim{pub first_lock_id:FirstLockId,pub actor:PlayerHash,pub party_hash:u64,pub proof:ProofHash,pub server_tick:u64}
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum WorldFirstVerdict{FirstRelic(RelicId),EchoRelic(RelicId),Invalid}
#[derive(Debug,Default)] pub struct FirstLockState{pub solved_mask:u16,pub proofs:[u64;12]}
impl FirstLockState{pub fn is_solved(&self,id:FirstLockId)->bool{id.0<12&&(self.solved_mask&(1u16<<id.0))!=0} pub fn mark_solved(&mut self,claim:WorldFirstClaim){if claim.first_lock_id.0<12{self.solved_mask|=1u16<<claim.first_lock_id.0;self.proofs[claim.first_lock_id.0 as usize]=claim.proof.0}}}
pub fn resolve_world_first(state:&mut FirstLockState,def:FirstLockDef,claim:WorldFirstClaim)->WorldFirstVerdict{if claim.first_lock_id!=def.id||claim.proof.0==0{return WorldFirstVerdict::Invalid} if state.is_solved(def.id){WorldFirstVerdict::EchoRelic(def.echo_relic)}else{state.mark_solved(claim);WorldFirstVerdict::FirstRelic(def.world_first_relic)}}
pub fn first_lock_proof(def:FirstLockDef,actor:PlayerHash,party_hash:u64,event_hash:EventHash,artifact:ArtifactHash,server_tick:u64)->ProofHash{proof_hash(&[def.id.0 as u64,def.account as u64,actor.0,party_hash,event_hash.0,artifact.0,server_tick])}
