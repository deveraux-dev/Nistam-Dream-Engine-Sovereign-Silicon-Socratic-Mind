//! Server-signed proof model shell.
#![allow(dead_code)]
use crate::lore_core::*; use crate::lore_registry::FirstLockId;
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum SignedProofKind{DeathScar,PuzzleScar,FirstRelic,EchoRelic,ArtifactProvenance,TcgCard,VowlessBlank}
#[derive(Debug,Clone,Copy)] pub struct ProofPreimage{pub kind:SignedProofKind,pub world:WorldHash,pub actor:PlayerHash,pub subject_hash:u64,pub tick:u64,pub payload_hash:u64}
#[derive(Debug,Clone,Copy)] pub struct ServerSignature{pub key_id:u32,pub signature_hi:u64,pub signature_lo:u64}
#[derive(Debug,Clone,Copy)] pub struct SignedLoreProof{pub preimage:ProofPreimage,pub proof:ProofHash,pub signature:ServerSignature}
pub fn proof_preimage_hash(p:ProofPreimage)->ProofHash{proof_hash(&[p.kind as u64,p.world.0,p.actor.0,p.subject_hash,p.tick,p.payload_hash])}
pub fn make_unsigned_proof(preimage:ProofPreimage)->SignedLoreProof{SignedLoreProof{preimage,proof:proof_preimage_hash(preimage),signature:ServerSignature{key_id:0,signature_hi:0,signature_lo:0}}}
pub fn first_lock_payload(id:FirstLockId,party_hash:u64,solver_count:u32)->u64{proof_hash(&[id.0 as u64,party_hash,solver_count as u64]).0}
