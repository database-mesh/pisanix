// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use openssl::{
    asn1::Asn1Time,
    hash::MessageDigest,
    nid::Nid,
    pkcs12::Pkcs12,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{extension::KeyUsage, X509Name, X509},
};

pub fn make_pkcs12() -> (Rsa<Private>, PKey<Private>, Vec<u8>) {
    let subject_name = "ns.pisa-proxy.io";

    let rsa_key = Rsa::generate(2048).unwrap();
    let pub_key = PKey::from_rsa(rsa_key.clone()).unwrap();

    let mut name = X509Name::builder().unwrap();
    name.append_entry_by_nid(Nid::COMMONNAME, subject_name).unwrap();
    let name = name.build();

    let key_usage = KeyUsage::new().digital_signature().build().unwrap();

    let mut builder = X509::builder().unwrap();
    builder.set_version(2).unwrap();
    builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
    builder.set_not_after(&Asn1Time::days_from_now(3650).unwrap()).unwrap();
    builder.set_subject_name(&name).unwrap();
    builder.set_issuer_name(&name).unwrap();
    builder.append_extension(key_usage).unwrap();
    builder.set_pubkey(&pub_key).unwrap();

    builder.sign(&pub_key, MessageDigest::sha256()).unwrap();
    let cert = builder.build();

    let pkcs12_builder = Pkcs12::builder();
    let pkcs12 = pkcs12_builder.build("pisa-proxy", subject_name, &pub_key, &cert).unwrap();
    let der = pkcs12.to_der().unwrap();

    (rsa_key, pub_key, der)
}
