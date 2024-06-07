use bitcoin::{
    consensus::serialize,
    hashes::{sha256d, Hash},
    Block, Txid,
};

use crate::{
    l1::{L1Tx, L1TxProof},
    prelude::Buf32,
};

// TODO: this should probably be proof for witness, it is proof for txns right now
fn get_cohashes_from_txids(txids: &[Txid], index: u32) -> Vec<Buf32> {
    assert!((index as usize) < txids.len());

    let mut curr_level: Vec<_> = txids.iter().cloned().map(|x| x.to_byte_array()).collect();
    let mut curr_index = index;
    let mut proof = Vec::new();

    while curr_level.len() > 1 {
        let len = curr_level.len();
        if len % 2 != 0 {
            curr_level.push(curr_level[len - 1].clone());
        }

        let proof_item_index = if curr_index % 2 == 0 {
            curr_index + 1
        } else {
            curr_index - 1
        };

        let mut item = curr_level[proof_item_index as usize];
        item.reverse(); // hash is stored in reverse order, so need to reverse here
        let proof_item = Buf32(item.into());
        proof.push(proof_item);

        // construct pairwise hash
        curr_level = curr_level
            .chunks(2)
            .map(|pair| {
                let [a, b] = pair else {
                    panic!("should be pair");
                };
                let mut arr = [0u8; 64];
                arr[..32].copy_from_slice(a);
                arr[32..].copy_from_slice(b);
                sha256d::Hash::hash(&arr).as_byte_array().clone()
            })
            .collect::<Vec<_>>();
        curr_index = curr_index >> 1;
    }
    proof
}

pub fn generate_l1_tx(idx: u32, block: &Block) -> L1Tx {
    assert!((idx as usize) < block.txdata.len());
    let tx = &block.txdata[idx as usize];

    let cohashes = get_cohashes_from_txids(
        &block
            .txdata
            .iter()
            .map(|x| x.compute_txid())
            .collect::<Vec<_>>(),
        idx,
    );

    let proof = L1TxProof::new(idx, cohashes);
    let tx = serialize(tx);

    L1Tx::new(proof, tx)
}

#[cfg(test)]
mod tests {
    use bitcoin::{consensus::deserialize, hex::DisplayHex};

    use super::*;

    fn get_block() -> Block {
        let rawblock = "00000020cecf04d0280eb8a70748d242373fd6f6a3f6df9e9c0f1e97a07c010ef5b3fa7a23db35e71f490b912741b861e71ae1c24f8eb7120a0279043e31473bf13ce5def6b75e66ffff7f20020000000b020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff04025b0200ffffffff023496a0120000000016001443822432e7df09da4247ccdba27ac3597fa8bd380000000000000000266a24aa21a9ed144eec290dc2b7f7ba875f81d5be40cd2c49003393393c81da674a1257898d62012000000000000000000000000000000000000000000000000000000000000000000000000002000000000101714163fc0f13ab22b1f407edecf7b65d780ed096edd1bac1e58ffe5b44d0abcf0000000000fdffffff023e22a82400000000160014f1d37dd4055fcbe663ef34e9f6de92bde94f5aec8096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db0247304402203a3c25d8cca9282dc5faf4edb72a6411b51466d970ecab149d67e3df2dd4055302201ca58eded3b03e228c2bc64fa3e265f5b5029bbc50e8d1bae02d2b4a0f378ea50121026cb9a8a39edd90d54aae826150c96f86d19dd64dac5d92143fe1a7d5bb1b30060000000002000000000101365109d23ee09e254a365ec61fd33fa5c3a9c5fd231df2124bb93f7dc8c318b30000000000fdffffff028096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db3e22a82400000000160014ed1dc9691a46af585c4759bcacdfbf87043a6bf30247304402200ca800d96d58ba4bd72da805c4bbbdc44c31aae482392dd0881e94e191699ce402206321e62be182d5ce2f83ef2046607b25647db6ec9cba7b356868ff1dda9e5c790121024e46a75f0d36817842c253bc107b239e7b2c1fe2aff72b9357640d30e8a849860000000002000000000101ee1de4c3d8d7bd9a9d025c5f53efbd150ea0b43a9e3777b41d675df4cc918cfe0000000000fdffffff023e22a82400000000160014c8b8179be4c0f4db9641276f2f2a89d00d5f254d8096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db02473044022056676e1b0b87a1b1bd2af00d8188308f442093391cdb888c9f2f6838d9c5c652022027f16f8023c88362359fca6c5064aa4329c78f8dca959dd23c048d7f877a52250121028ad55e57f2128976267b995ddf12d0f6630da20da1ff3c1bf47edfa84f658a08000000000200000000010145684300d33ed90afbba16fd041f5b12961072902105239b4a75604376387f380000000000fdffffff023e22a824000000001600144b4690c37c400f5a9322044aed2c2d855f24ba3c8096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db024730440220447d5451fac88c73ebe94e04e2d5bf591c15b4368dc09ea417b6849bcc7644a00220468749760103dd178a39215157ae9d554f20de7c9fbf8fe4269ea343d5bf80f5012103de5318151e4d19ed71f9d8e69c98df1a1272c8c8e11305bca57d9076fd7581d60000000002000000000101a50d3a129ed5858084abf67649a9ff0db4ab1b793cd37ed2086a3015e056f06f0000000000fdffffff028096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db3e22a824000000001600147697f0ab4555d849eda4f4edc600140721a24baa0247304402203906786c4758d0917dc113a93b46bd496c8c0026ae97bd3e71f4b0b2e4406e8c0220601eae276d166fbe0f31940c0e4d93275e3db490889549d2ed3cdf2189a2c9e6012103c920b89d758f26fcf213b6b097f9cf45de362717bf1467879f1f0f6cc35fb10400000000020000000001015ca5229ad9315604ee0efdd8f0c5a5e646805f49b5f6d82497603669421a0d660000000000fdffffff028096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db3e22a82400000000160014ff0d6af0f9c38f736a0ae3ccb53e62882c1368aa0247304402201f6376beedfd2b46b8826cdc8739bfd730d4e6be2eed5e2a4c6918bafb3fad52022075b7154d1e6380b35394711575ab0de5fb7f6dc9111f8096dfce43c70111402e012102affe33fa0c53b734fd128561d35a053cfa14c122a76ae97bc71ec9eb2553711700000000020000000001014b37ecbeea26538433e6c9c96ac304d84dd1558236b400d222bb4cef1502a68c0000000000fdffffff028096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db3e22a82400000000160014242e68f1fe8fa1012ae2ade473a5d95d7630abee0247304402206a3fefd6287b023dab4e7f3eecb6680ee65344ac038851f637595211a8f8436002204039727369556469ae4f26aecf638469d23af4656de06a6316b1bb1b10c47b58012102370de05670d4327d4e68db0e72afaf1445e7df65772e8b7940a59004990f465600000000020000000001015798abe83d57cf75f9eb8b07cd22cc2730051f36b8ea9292471e3c50c47b081e0000000000fdffffff028096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db3e22a8240000000016001428194fec04bfcc757d39d6207af79780ae02044b02473044022005e73efb4634edd9ce50e5fce4326331d3ea10e09e02c9bf2e06786ca864280202205806bacc496dd2f16a2bbd8cc7de35a46311789614b4c3d78004eeda0ffda452012103a1b9d92159960dd69674a0663ca33e981d971bc383a8238408132d4f2844713f0000000002000000000101810d179e15c4c028ac5bad124d54df0fe6e4e107021aa47b416d9c3bf523a53d0000000000fdffffff023e22a82400000000160014d51914794c7ed9b6b8ed0f5fd55cbcb7fcaec3e18096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db024730440220514ed633f8803920a5467ed30378cb7dd96d3ce8e193b87b1269919ec86699940220100833b84ae1253bd0a2846014a0dcca55ae103aa694c046cf294589447ab23c0121028a2b1479601116997cf1a6429f0383c4fe32c393a5549a32f0eb4f4e8e31b59300000000020000000001011e63aea9f6b294a2214ea7fb8d88a9adb63e6e9387b5964c67213880e092ccea0000000000fdffffff023e22a824000000001600144bb6004ee3c5789f6696e3456ebf3bb0ec7d1dcf8096980000000000160014ffd7e590dee29cf5e793113e0ebf6cf70aaf35db024730440220116f78659bfa970076a06adffa7ff6edf1c1c4a6669cee224aa4482f996909dd02203c0cee047d59a2bdcf05f323965602525ef69e5967b9c4ce4b9f84cfed5169110121039a8363b2023ea524edaa0a3f735b21f458d324648b1979f44f71bb36ec59828500000000";
        // The block contains the following txids
        //     "ae43d3b04dfaf6093d801f82997d10ac9e59fa27b96e054ffb65d6f5b03e95d8", // coinbase
        //     "8eafdfed11ae5876c10a128c68d857f6373511932bdb012ade2285c9d9f4ff2a",
        //     "0f14ba573cc24912feb1566e4a2308b86f55fe79964288ff400b818938b6ae38",
        //     "d95eedc92e26910f7dac62cbeee4b713605d5e24e7f569b10cbc6cbda96e6652",
        //     "eade6995f6668be73fe9b90c4d20c657b28b0b64530d9c121f3f47feed9c8653",
        //     "10a223a34143031ba8a42f2c8adf9bcfb15b67e820c55d750c308702f5c85154",
        //     "9d0ea944cdae406f656a3d277c7ad1b7914e2e760458ac40cd68a8a936ad7054",
        //     "5bc67f1d847b4f232b8ad385e59264ae5ee8da2e3eeb4ac0aee283c5ba241864",
        //     "bc036c1a9550e2c1eb1c1b3727b813900b6223639539f37cc5b5ef71d5e72770",
        //     "48897fd2acbf90474224424290813b18924fbeb99812a2cd3264d9a136edd8ef",
        //     "68aa9db5d50385634a3a5b9927b214fe8ee9ab85cb421e2b819f7c6131dc9af7",
        deserialize(&hex::decode(rawblock).unwrap()).unwrap()
    }

    #[test]
    fn test_txdata_to_l1tx() {
        let block: Block = get_block();
        let idx = 7; // corresponding to 5bc67f1d847b4f232b8ad385e59264ae5ee8da2e3eeb4ac0aee283c5ba241864
        let l1_tx = generate_l1_tx(idx, &block).unwrap();
        let exp_cohashes: Vec<_> = [
            "9d0ea944cdae406f656a3d277c7ad1b7914e2e760458ac40cd68a8a936ad7054",
            "b4b33efa721b091ae146ce7ce93d81f6d7974587ab0f566b03019fefe58c86c7",
            "09653680f150d2ea72a3b3a0bb00ce60deb3dc6b410b2a9cbbb953a70ece5092",
            "69224097981b0bd49c45ff28a03b88d2c6dea04e9e6bd0e5bbbe6b6ff6ccf0e4",
        ]
        .iter()
        .map(|x| {
            let mut arr = [0u8; 32];
            let decoded = hex::decode(x).unwrap();
            println!("decoded: {:?}", decoded.to_lower_hex_string());
            arr.copy_from_slice(&decoded);
            Buf32(arr.into())
        })
        .collect();
        assert_eq!(l1_tx.proof().position(), idx);
        assert_eq!(*l1_tx.proof().cohashes(), exp_cohashes);
    }
}
