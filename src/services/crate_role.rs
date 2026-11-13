use serde::Deserialize;
#[derive(Clone,Copy,Deserialize)]#[serde(rename_all="UPPERCASE")]pub enum Role{Admin,User}
impl Role{pub fn as_str(self)->&'static str{match self{Self::Admin=>"ADMIN",Self::User=>"USER"}}}
