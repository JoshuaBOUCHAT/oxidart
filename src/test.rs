use bytes::Bytes;

use crate::OxidArt;

#[test]
fn test_get_set_basic() {
    let mut art = OxidArt::new();
    let key = Bytes::from_static(b"Joshua");
    let val = Bytes::from_static(b"BOUCHAT");
    art.set(key.clone(), val.clone());
    assert_eq!(art.get(key), Some(val));
}

#[test]
fn test_empty_key() {
    let mut art = OxidArt::new();
    let key = Bytes::from_static(b"");
    let val = Bytes::from_static(b"root_value");
    art.set(key.clone(), val.clone());
    assert_eq!(art.get(key), Some(val));
}

#[test]
fn test_get_nonexistent() {
    let mut art = OxidArt::new();
    assert_eq!(art.get(Bytes::from_static(b"missing")), None);
}

#[test]
fn test_overwrite_value() {
    let mut art = OxidArt::new();
    let key = Bytes::from_static(b"key");
    let val1 = Bytes::from_static(b"value1");
    let val2 = Bytes::from_static(b"value2");

    art.set(key.clone(), val1.clone());
    assert_eq!(art.get(key.clone()), Some(val1));

    art.set(key.clone(), val2.clone());
    assert_eq!(art.get(key), Some(val2));
}

#[test]
fn test_common_prefix_split() {
    // Test le split: "user" et "uso" partagent "us"
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"val_user"));
    art.set(Bytes::from_static(b"uso"), Bytes::from_static(b"val_uso"));

    assert_eq!(
        art.get(Bytes::from_static(b"user")),
        Some(Bytes::from_static(b"val_user"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"uso")),
        Some(Bytes::from_static(b"val_uso"))
    );
    // "us" n'a pas de valeur
    assert_eq!(art.get(Bytes::from_static(b"us")), None);
}

#[test]
fn test_prefix_is_also_key() {
    // "us" est un préfixe de "user" mais aussi une clé
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"val_user"));
    art.set(Bytes::from_static(b"us"), Bytes::from_static(b"val_us"));

    assert_eq!(
        art.get(Bytes::from_static(b"user")),
        Some(Bytes::from_static(b"val_user"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"us")),
        Some(Bytes::from_static(b"val_us"))
    );
}

#[test]
fn test_multiple_branches() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"apple"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"application"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"banana"), Bytes::from_static(b"3"));
    art.set(Bytes::from_static(b"band"), Bytes::from_static(b"4"));

    assert_eq!(
        art.get(Bytes::from_static(b"apple")),
        Some(Bytes::from_static(b"1"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"application")),
        Some(Bytes::from_static(b"2"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"banana")),
        Some(Bytes::from_static(b"3"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"band")),
        Some(Bytes::from_static(b"4"))
    );

    // Clés partielles qui n'existent pas
    assert_eq!(art.get(Bytes::from_static(b"app")), None);
    assert_eq!(art.get(Bytes::from_static(b"ban")), None);
}

#[test]
fn test_del_basic() {
    let mut art = OxidArt::new();
    let key = Bytes::from_static(b"hello");
    let val = Bytes::from_static(b"world");

    art.set(key.clone(), val.clone());
    assert_eq!(art.get(key.clone()), Some(val.clone()));

    let deleted = art.del(key.clone());
    assert_eq!(deleted, Some(val));
    assert_eq!(art.get(key), None);
}

#[test]
fn test_del_nonexistent() {
    let mut art = OxidArt::new();
    assert_eq!(art.del(Bytes::from_static(b"missing")), None);
}

#[test]
fn test_del_empty_key() {
    let mut art = OxidArt::new();
    let val = Bytes::from_static(b"root");

    art.set(Bytes::from_static(b""), val.clone());
    assert_eq!(art.del(Bytes::from_static(b"")), Some(val));
    assert_eq!(art.get(Bytes::from_static(b"")), None);
}

#[test]
fn test_del_with_recompression() {
    // us -> {er, o}  après del("uso") -> "user"
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"val_user"));
    art.set(Bytes::from_static(b"uso"), Bytes::from_static(b"val_uso"));

    // Supprimer "uso"
    let deleted = art.del(Bytes::from_static(b"uso"));
    assert_eq!(deleted, Some(Bytes::from_static(b"val_uso")));

    // "user" doit toujours exister
    assert_eq!(
        art.get(Bytes::from_static(b"user")),
        Some(Bytes::from_static(b"val_user"))
    );
    // "uso" n'existe plus
    assert_eq!(art.get(Bytes::from_static(b"uso")), None);
}

#[test]
fn test_del_intermediate_node_with_children() {
    // Supprimer un node intermédiaire qui a des enfants
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"val_a"));
    art.set(Bytes::from_static(b"ab"), Bytes::from_static(b"val_ab"));
    art.set(Bytes::from_static(b"abc"), Bytes::from_static(b"val_abc"));

    // Supprimer "ab" qui est intermédiaire
    let deleted = art.del(Bytes::from_static(b"ab"));
    assert_eq!(deleted, Some(Bytes::from_static(b"val_ab")));

    // "a" et "abc" doivent toujours exister
    assert_eq!(
        art.get(Bytes::from_static(b"a")),
        Some(Bytes::from_static(b"val_a"))
    );
    assert_eq!(
        art.get(Bytes::from_static(b"abc")),
        Some(Bytes::from_static(b"val_abc"))
    );
    assert_eq!(art.get(Bytes::from_static(b"ab")), None);
}

#[test]
fn test_many_keys_same_prefix() {
    let mut art = OxidArt::new();

    // Beaucoup de clés avec le même préfixe pour tester les huge_childs
    for i in 0..20u8 {
        let key = Bytes::from(vec![b'x', i]);
        let val = Bytes::from(vec![i]);
        art.set(key, val);
    }

    for i in 0..20u8 {
        let key = Bytes::from(vec![b'x', i]);
        let expected = Bytes::from(vec![i]);
        assert_eq!(art.get(key), Some(expected));
    }
}

#[test]
fn test_long_keys() {
    let mut art = OxidArt::new();

    let key1 = Bytes::from(vec![b'a'; 100]);
    let key2 = Bytes::from(vec![b'a'; 50]);
    let val1 = Bytes::from_static(b"long");
    let val2 = Bytes::from_static(b"medium");

    art.set(key1.clone(), val1.clone());
    art.set(key2.clone(), val2.clone());

    assert_eq!(art.get(key1), Some(val1));
    assert_eq!(art.get(key2), Some(val2));
}

#[test]
fn test_del_then_reinsert() {
    let mut art = OxidArt::new();
    let key = Bytes::from_static(b"key");
    let val1 = Bytes::from_static(b"val1");
    let val2 = Bytes::from_static(b"val2");

    art.set(key.clone(), val1.clone());
    art.del(key.clone());
    art.set(key.clone(), val2.clone());

    assert_eq!(art.get(key), Some(val2));
}

#[test]
fn test_del_all_keys() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"b"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"c"), Bytes::from_static(b"3"));

    art.del(Bytes::from_static(b"a"));
    art.del(Bytes::from_static(b"b"));
    art.del(Bytes::from_static(b"c"));

    assert_eq!(art.get(Bytes::from_static(b"a")), None);
    assert_eq!(art.get(Bytes::from_static(b"b")), None);
    assert_eq!(art.get(Bytes::from_static(b"c")), None);
}

#[test]
fn test_partial_key_not_found() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"hello_world"),
        Bytes::from_static(b"val"),
    );

    // Clés partielles ne doivent pas matcher
    assert_eq!(art.get(Bytes::from_static(b"hello")), None);
    assert_eq!(art.get(Bytes::from_static(b"hello_")), None);
    assert_eq!(art.get(Bytes::from_static(b"hello_worl")), None);
    // Clé trop longue non plus
    assert_eq!(art.get(Bytes::from_static(b"hello_world!")), None);
}

// ============ Tests pour getn ============

#[test]
fn test_getn_basic() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_data"),
    );
    art.set(
        Bytes::from_static(b"user:bob"),
        Bytes::from_static(b"bob_data"),
    );
    art.set(
        Bytes::from_static(b"user:charlie"),
        Bytes::from_static(b"charlie_data"),
    );
    art.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"post_1"));

    let results = art.getn(Bytes::from_static(b"user:"));

    assert_eq!(results.len(), 3);
    assert!(results.contains(&(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_data")
    )));
    assert!(results.contains(&(
        Bytes::from_static(b"user:bob"),
        Bytes::from_static(b"bob_data")
    )));
    assert!(results.contains(&(
        Bytes::from_static(b"user:charlie"),
        Bytes::from_static(b"charlie_data")
    )));
}

#[test]
fn test_getn_empty_prefix() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"b"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"c"), Bytes::from_static(b"3"));

    let results = art.getn(Bytes::from_static(b""));

    assert_eq!(results.len(), 3);
}

#[test]
fn test_getn_no_match() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"data"),
    );

    let results = art.getn(Bytes::from_static(b"post:"));

    assert!(results.is_empty());
}

#[test]
fn test_getn_exact_key() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"));
    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_val"),
    );

    // Préfixe exact "user" doit retourner "user" et "user:alice"
    let results = art.getn(Bytes::from_static(b"user"));

    assert_eq!(results.len(), 2);
    assert!(results.contains(&(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"))));
    assert!(results.contains(&(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_val")
    )));
}

#[test]
fn test_getn_prefix_in_compression() {
    // Test quand le préfixe se termine au milieu d'une compression
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"application"),
        Bytes::from_static(b"app_val"),
    );

    // "app" est un préfixe de "application"
    let results = art.getn(Bytes::from_static(b"app"));

    assert_eq!(results.len(), 1);
    assert!(results.contains(&(
        Bytes::from_static(b"application"),
        Bytes::from_static(b"app_val")
    )));
}

#[test]
fn test_getn_with_nested_keys() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"ab"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"abc"), Bytes::from_static(b"3"));
    art.set(Bytes::from_static(b"abcd"), Bytes::from_static(b"4"));
    art.set(Bytes::from_static(b"abd"), Bytes::from_static(b"5"));

    let results = art.getn(Bytes::from_static(b"ab"));

    assert_eq!(results.len(), 4); // ab, abc, abcd, abd
    assert!(results.contains(&(Bytes::from_static(b"ab"), Bytes::from_static(b"2"))));
    assert!(results.contains(&(Bytes::from_static(b"abc"), Bytes::from_static(b"3"))));
    assert!(results.contains(&(Bytes::from_static(b"abcd"), Bytes::from_static(b"4"))));
    assert!(results.contains(&(Bytes::from_static(b"abd"), Bytes::from_static(b"5"))));
}

#[test]
fn test_getn_single_char_prefix() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"aa"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"ab"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"ba"), Bytes::from_static(b"3"));

    let results = art.getn(Bytes::from_static(b"a"));

    assert_eq!(results.len(), 2);
    assert!(results.contains(&(Bytes::from_static(b"aa"), Bytes::from_static(b"1"))));
    assert!(results.contains(&(Bytes::from_static(b"ab"), Bytes::from_static(b"2"))));
}

#[test]
fn test_getn_many_children() {
    let mut art = OxidArt::new();

    // Plus de 10 enfants pour tester huge_childs
    for i in 0..20u8 {
        let key = Bytes::from(vec![b'x', b':', i]);
        let val = Bytes::from(vec![i]);
        art.set(key, val);
    }

    let results = art.getn(Bytes::from_static(b"x:"));

    assert_eq!(results.len(), 20);
}

// ============ Tests pour deln ============

#[test]
fn test_deln_basic() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_data"),
    );
    art.set(
        Bytes::from_static(b"user:bob"),
        Bytes::from_static(b"bob_data"),
    );
    art.set(
        Bytes::from_static(b"user:charlie"),
        Bytes::from_static(b"charlie_data"),
    );
    art.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"post_1"));

    let deleted = art.deln(Bytes::from_static(b"user:"));

    assert_eq!(deleted, 3);
    assert_eq!(art.get(Bytes::from_static(b"user:alice")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:bob")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:charlie")), None);
    // post:1 doit toujours exister
    assert_eq!(
        art.get(Bytes::from_static(b"post:1")),
        Some(Bytes::from_static(b"post_1"))
    );
}

#[test]
fn test_deln_empty_prefix() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"b"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"c"), Bytes::from_static(b"3"));

    let deleted = art.deln(Bytes::from_static(b""));

    assert_eq!(deleted, 3);
    assert_eq!(art.get(Bytes::from_static(b"a")), None);
    assert_eq!(art.get(Bytes::from_static(b"b")), None);
    assert_eq!(art.get(Bytes::from_static(b"c")), None);
}

#[test]
fn test_deln_no_match() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"data"),
    );

    let deleted = art.deln(Bytes::from_static(b"post:"));

    assert_eq!(deleted, 0);
    // user:alice doit toujours exister
    assert_eq!(
        art.get(Bytes::from_static(b"user:alice")),
        Some(Bytes::from_static(b"data"))
    );
}

#[test]
fn test_deln_exact_key_with_children() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"));
    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"alice_val"),
    );
    art.set(
        Bytes::from_static(b"user:bob"),
        Bytes::from_static(b"bob_val"),
    );

    // Supprimer "user" et tous ses descendants
    let deleted = art.deln(Bytes::from_static(b"user"));

    assert_eq!(deleted, 3);
    assert_eq!(art.get(Bytes::from_static(b"user")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:alice")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:bob")), None);
}

#[test]
fn test_deln_prefix_in_compression() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"application"),
        Bytes::from_static(b"app_val"),
    );
    art.set(
        Bytes::from_static(b"apple"),
        Bytes::from_static(b"apple_val"),
    );

    // "app" est un préfixe commun
    let deleted = art.deln(Bytes::from_static(b"app"));

    assert_eq!(deleted, 2);
    assert_eq!(art.get(Bytes::from_static(b"application")), None);
    assert_eq!(art.get(Bytes::from_static(b"apple")), None);
}

#[test]
fn test_deln_with_nested_keys() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"a"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"ab"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"abc"), Bytes::from_static(b"3"));
    art.set(Bytes::from_static(b"abcd"), Bytes::from_static(b"4"));
    art.set(Bytes::from_static(b"abd"), Bytes::from_static(b"5"));
    art.set(Bytes::from_static(b"b"), Bytes::from_static(b"6"));

    let deleted = art.deln(Bytes::from_static(b"ab"));

    assert_eq!(deleted, 4); // ab, abc, abcd, abd
    assert_eq!(
        art.get(Bytes::from_static(b"a")),
        Some(Bytes::from_static(b"1"))
    );
    assert_eq!(art.get(Bytes::from_static(b"ab")), None);
    assert_eq!(art.get(Bytes::from_static(b"abc")), None);
    assert_eq!(
        art.get(Bytes::from_static(b"b")),
        Some(Bytes::from_static(b"6"))
    );
}

#[test]
fn test_deln_many_children() {
    let mut art = OxidArt::new();

    // Plus de 10 enfants pour tester huge_childs
    for i in 0..20u8 {
        let key = Bytes::from(vec![b'x', b':', i]);
        let val = Bytes::from(vec![i]);
        art.set(key, val);
    }

    let deleted = art.deln(Bytes::from_static(b"x:"));

    assert_eq!(deleted, 20);

    // Vérifier qu'ils sont tous supprimés
    for i in 0..20u8 {
        let key = Bytes::from(vec![b'x', b':', i]);
        assert_eq!(art.get(key), None);
    }
}

#[test]
fn test_deln_then_insert() {
    let mut art = OxidArt::new();

    art.set(
        Bytes::from_static(b"user:alice"),
        Bytes::from_static(b"old"),
    );
    art.deln(Bytes::from_static(b"user:"));

    // Réinsérer après suppression
    art.set(Bytes::from_static(b"user:bob"), Bytes::from_static(b"new"));

    assert_eq!(art.get(Bytes::from_static(b"user:alice")), None);
    assert_eq!(
        art.get(Bytes::from_static(b"user:bob")),
        Some(Bytes::from_static(b"new"))
    );
}

#[test]
fn test_deln_partial_match() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"hello"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"help"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"world"), Bytes::from_static(b"3"));

    // "hel" matche "hello" et "help"
    let deleted = art.deln(Bytes::from_static(b"hel"));

    assert_eq!(deleted, 2);
    assert_eq!(art.get(Bytes::from_static(b"hello")), None);
    assert_eq!(art.get(Bytes::from_static(b"help")), None);
    assert_eq!(
        art.get(Bytes::from_static(b"world")),
        Some(Bytes::from_static(b"3"))
    );
}

// ============ Tests TTL ============

#[cfg(feature = "ttl")]
#[test]
fn test_ttl_expired_on_get() {
    use std::time::Duration;

    let mut art = OxidArt::new();
    art.set_now(0);

    // Insert with short TTL (1 second)
    art.set_ttl(Bytes::from_static(b"expired"), Duration::from_secs(1), Bytes::from_static(b"old"));
    // Insert with longer TTL (100 seconds)
    art.set_ttl(Bytes::from_static(b"valid"), Duration::from_secs(100), Bytes::from_static(b"new"));
    // Insert with no expiry
    art.set(Bytes::from_static(b"forever"), Bytes::from_static(b"eternal"));

    // Move time forward past first TTL
    art.set_now(50);

    // Expired key should return None and be cleaned up
    assert_eq!(art.get(Bytes::from_static(b"expired")), None);
    // Valid key should still work
    assert_eq!(
        art.get(Bytes::from_static(b"valid")),
        Some(Bytes::from_static(b"new"))
    );
    // No expiry key should work
    assert_eq!(
        art.get(Bytes::from_static(b"forever")),
        Some(Bytes::from_static(b"eternal"))
    );

    // Move time forward, valid should expire
    art.set_now(150);
    assert_eq!(art.get(Bytes::from_static(b"valid")), None);
    // No expiry still works
    assert_eq!(
        art.get(Bytes::from_static(b"forever")),
        Some(Bytes::from_static(b"eternal"))
    );
}

#[cfg(feature = "ttl")]
#[test]
fn test_ttl_getn_filters_expired() {
    use std::time::Duration;

    let mut art = OxidArt::new();
    art.set_now(0);

    // Short TTL - will expire
    art.set_ttl(Bytes::from_static(b"user:expired"), Duration::from_secs(1), Bytes::from_static(b"old"));
    // Longer TTL - won't expire
    art.set_ttl(Bytes::from_static(b"user:valid"), Duration::from_secs(100), Bytes::from_static(b"new"));
    // No expiry
    art.set(Bytes::from_static(b"user:forever"), Bytes::from_static(b"eternal"));

    // Move time forward past first TTL
    art.set_now(50);

    let results = art.getn(Bytes::from_static(b"user:"));

    // Should only return 2 (valid and forever), not the expired one
    assert_eq!(results.len(), 2);
    assert!(!results.iter().any(|(k, _)| k == &Bytes::from_static(b"user:expired")));
    assert!(results.iter().any(|(k, _)| k == &Bytes::from_static(b"user:valid")));
    assert!(results.iter().any(|(k, _)| k == &Bytes::from_static(b"user:forever")));
}

#[cfg(feature = "ttl")]
#[test]
fn test_ttl_cleanup_on_expired_get() {
    use std::time::Duration;

    let mut art = OxidArt::new();
    art.set_now(0);

    // Create a path: user -> er (with short TTL)
    art.set_ttl(Bytes::from_static(b"user"), Duration::from_secs(1), Bytes::from_static(b"expired_user"));
    // Longer TTL
    art.set_ttl(Bytes::from_static(b"username"), Duration::from_secs(100), Bytes::from_static(b"valid"));

    // Move time forward
    art.set_now(50);

    // Get the expired key - should trigger cleanup
    assert_eq!(art.get(Bytes::from_static(b"user")), None);

    // The valid key should still work
    assert_eq!(
        art.get(Bytes::from_static(b"username")),
        Some(Bytes::from_static(b"valid"))
    );
}

#[cfg(feature = "ttl")]
#[test]
fn test_evict_expired_basic() {
    use std::time::Duration;

    let mut art = OxidArt::new();
    art.set_now(0);

    // Insert 50 keys with short TTL
    for i in 0..50u8 {
        let key = Bytes::from(vec![b'k', i]);
        art.set_ttl(key, Duration::from_secs(1), Bytes::from_static(b"val"));
    }

    // Insert 10 keys with long TTL
    for i in 0..10u8 {
        let key = Bytes::from(vec![b'l', i]);
        art.set_ttl(key, Duration::from_secs(1000), Bytes::from_static(b"val"));
    }

    // Insert 10 keys without TTL
    for i in 0..10u8 {
        let key = Bytes::from(vec![b'n', i]);
        art.set(key, Bytes::from_static(b"val"));
    }

    // Move time forward - short TTL keys are now expired
    art.set_now(100);

    // Evict expired entries (may need multiple calls due to probabilistic sampling)
    let mut total_evicted = 0;
    for _ in 0..10 {
        let evicted = art.evict_expired();
        total_evicted += evicted;
        if evicted == 0 {
            break;
        }
    }

    // Should have evicted all 50 expired keys
    assert_eq!(total_evicted, 50);

    // Long TTL keys should still exist
    for i in 0..10u8 {
        let key = Bytes::from(vec![b'l', i]);
        assert_eq!(art.get(key), Some(Bytes::from_static(b"val")));
    }

    // No TTL keys should still exist
    for i in 0..10u8 {
        let key = Bytes::from(vec![b'n', i]);
        assert_eq!(art.get(key), Some(Bytes::from_static(b"val")));
    }
}

#[cfg(feature = "ttl")]
#[test]
fn test_evict_expired_partial() {
    use std::time::Duration;

    let mut art = OxidArt::new();
    art.set_now(0);

    // Insert 10 keys with short TTL (will expire)
    for i in 0..10u8 {
        let key = Bytes::from(vec![b'e', i]);
        art.set_ttl(key, Duration::from_secs(1), Bytes::from_static(b"val"));
    }

    // Insert 90 keys with long TTL (won't expire)
    for i in 0..90u8 {
        let key = Bytes::from(vec![b'v', i]);
        art.set_ttl(key, Duration::from_secs(1000), Bytes::from_static(b"val"));
    }

    // Move time forward
    art.set_now(100);

    // Evict - should stop after one round since < 25% expired
    let evicted = art.evict_expired();

    // Should have evicted at most the 10 expired keys
    assert!(evicted <= 10);
}

// ============ Tests avec dictionnaire français ============
