//! Canonical lore core: de-duplicate earlier sidecars.
#![allow(dead_code)]
pub type Permyriad=i32;
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct PlayerHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct EntityHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct EventHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct ZoneHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct ArtifactHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub struct ProofHash(pub u64);
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]#[repr(u8)]
pub enum HiddenAccount{RedDebt=0,StoneRoot=1,DoubleWitness=2,GraveWater=3,CrownlessRoar=4,CleanIndex=5,EqualKnife=6,VenomWedding=7,FarWound=8,LastToll=9,HollowStar=10,MercyDrowned=11,OutsideWheel=12}
impl HiddenAccount{pub fn index(self)->Option<u8>{if matches!(self,Self::OutsideWheel){None}else{Some(self as u8)}} pub fn from_index(i:u8)->Self{match i%12{0=>Self::RedDebt,1=>Self::StoneRoot,2=>Self::DoubleWitness,3=>Self::GraveWater,4=>Self::CrownlessRoar,5=>Self::CleanIndex,6=>Self::EqualKnife,7=>Self::VenomWedding,8=>Self::FarWound,9=>Self::LastToll,10=>Self::HollowStar,_=>Self::MercyDrowned}} pub fn label(self)->&'static str{match self{Self::RedDebt=>"Red Debt",Self::StoneRoot=>"Stone Root",Self::DoubleWitness=>"Double Witness",Self::GraveWater=>"Grave-Water",Self::CrownlessRoar=>"Crownless Roar",Self::CleanIndex=>"Clean Index",Self::EqualKnife=>"Equal Knife",Self::VenomWedding=>"Venom Wedding",Self::FarWound=>"Far Wound",Self::LastToll=>"Last Toll",Self::HollowStar=>"Hollow Star",Self::MercyDrowned=>"Mercy Drowned",Self::OutsideWheel=>"Outside the Wheel"}}}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum MechanicalDemand{Aggression,Patience,Mobility,StationaryChannel,InventoryPrecision,ParryTiming,Diplomacy,Crafting,Theft,WitnessBuilding,DeathRoute,Refusal}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum SieveGeometry{Direct,SuperiorDexter,TeteDeCharge,Quincunx,Yod,FingerOfGod,Vowless}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum AuthorityTemperament{Benefic,Malefic,Neutral}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)] pub enum AuthorityOutcome{None,Bonification,Maltreatment,Mitigation,Clash}
#[derive(Debug,Clone,Copy,Default)] pub struct SuperiorDexterContext{pub initiative_q:Permyriad,pub elevation_q:Permyriad,pub legal_q:Permyriad,pub witness_q:Permyriad,pub route_q:Permyriad,pub artifact_q:Permyriad,pub death_scar_q:Permyriad}
impl SuperiorDexterContext{pub fn score_q(self)->Permyriad{self.initiative_q+self.elevation_q+self.legal_q+self.witness_q+self.route_q+self.artifact_q+self.death_scar_q}}
#[derive(Debug,Clone,Copy)] pub struct TeteDeCharge{pub entity_hash:EntityHash,pub event_hash:EventHash,pub authority_q:Permyriad,pub protected_q:Permyriad,pub cut:bool}
impl TeteDeCharge{pub fn effective_q(self)->Permyriad{if self.cut{-self.authority_q.abs()}else{self.authority_q+self.protected_q/2}}}
#[derive(Debug,Clone,Copy)] pub struct QuincunxPressure{pub demand_a:MechanicalDemand,pub demand_b:MechanicalDemand,pub demand_a_satisfied:bool,pub demand_b_satisfied:bool,pub severity_q:Permyriad}
impl QuincunxPressure{pub fn pressure_q(self)->Permyriad{match(self.demand_a_satisfied,self.demand_b_satisfied){(true,true)=>0,(true,false)|(false,true)=>self.severity_q,(false,false)=>self.severity_q.saturating_mul(2)}}}
#[derive(Debug,Clone,Copy,Default)] pub struct YodPressure{pub base_a_q:Permyriad,pub base_b_q:Permyriad,pub apex_resilience_q:Permyriad,pub cooperation_q:Permyriad}
impl YodPressure{pub fn pressure_q(self)->Permyriad{(self.base_a_q+self.base_b_q+self.cooperation_q-self.apex_resilience_q).max(0)} pub fn finger_of_god_q(self,quincunx_q:Permyriad)->Permyriad{self.pressure_q()+quincunx_q/2}}
pub fn resolve_authority(score_q:Permyriad,t:AuthorityTemperament)->AuthorityOutcome{if score_q>0{match t{AuthorityTemperament::Benefic=>AuthorityOutcome::Bonification,AuthorityTemperament::Malefic=>AuthorityOutcome::Maltreatment,AuthorityTemperament::Neutral=>AuthorityOutcome::Mitigation}}else if score_q==0{AuthorityOutcome::Clash}else{AuthorityOutcome::None}}
pub fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h^(h>>32)} pub fn proof_hash(parts:&[u64])->ProofHash{let mut h=0xcbf29ce484222325u64;for p in parts{h=mix(h,*p)}ProofHash(h)}
