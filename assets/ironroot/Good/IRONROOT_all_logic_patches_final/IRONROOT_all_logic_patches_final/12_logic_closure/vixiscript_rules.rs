//! VixiScript asymmetric rule model.
#![allow(dead_code)]
use crate::lore_core::{MechanicalDemand,SieveGeometry};
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum VixiRuleOp{Add,Subtract,Require,Suppress,Expose,Downgrade,Cancel}
#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum VixiTarget{SuperiorDexter,TeteDeCharge,Quincunx,Yod,FingerOfGod,FirstLock,Echo,Vowless}
#[derive(Debug,Clone,Copy)] pub struct VixiRule{pub target:VixiTarget,pub op:VixiRuleOp,pub value_q:i32,pub demand_a:Option<MechanicalDemand>,pub demand_b:Option<MechanicalDemand>,pub geometry:SieveGeometry}
pub fn parse_target(t:&str)->Option<VixiTarget>{match t{"superior_dexter"|"dexter"=>Some(VixiTarget::SuperiorDexter),"tete_de_charge"|"tes_de_charge"|"charge_head"=>Some(VixiTarget::TeteDeCharge),"quincunx"=>Some(VixiTarget::Quincunx),"yod"=>Some(VixiTarget::Yod),"finger_of_god"=>Some(VixiTarget::FingerOfGod),"first_lock"=>Some(VixiTarget::FirstLock),"echo"=>Some(VixiTarget::Echo),"vowless"=>Some(VixiTarget::Vowless),_=>None}}
pub fn apply_rule(base_q:i32,rule:VixiRule)->i32{match rule.op{VixiRuleOp::Add=>base_q.saturating_add(rule.value_q),VixiRuleOp::Subtract=>base_q.saturating_sub(rule.value_q),VixiRuleOp::Suppress|VixiRuleOp::Cancel=>0,VixiRuleOp::Downgrade=>base_q/2,VixiRuleOp::Require|VixiRuleOp::Expose=>base_q}}
