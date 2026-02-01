use bytes::Bytes;

use crate::OxidArt;

const FRENCH_WORDS: &str = include_str!("../list.txt");

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
    let art = OxidArt::new();
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

    assert_eq!(art.get(Bytes::from_static(b"user")), Some(Bytes::from_static(b"val_user")));
    assert_eq!(art.get(Bytes::from_static(b"uso")), Some(Bytes::from_static(b"val_uso")));
    // "us" n'a pas de valeur
    assert_eq!(art.get(Bytes::from_static(b"us")), None);
}

#[test]
fn test_prefix_is_also_key() {
    // "us" est un préfixe de "user" mais aussi une clé
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"val_user"));
    art.set(Bytes::from_static(b"us"), Bytes::from_static(b"val_us"));

    assert_eq!(art.get(Bytes::from_static(b"user")), Some(Bytes::from_static(b"val_user")));
    assert_eq!(art.get(Bytes::from_static(b"us")), Some(Bytes::from_static(b"val_us")));
}

#[test]
fn test_multiple_branches() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"apple"), Bytes::from_static(b"1"));
    art.set(Bytes::from_static(b"application"), Bytes::from_static(b"2"));
    art.set(Bytes::from_static(b"banana"), Bytes::from_static(b"3"));
    art.set(Bytes::from_static(b"band"), Bytes::from_static(b"4"));

    assert_eq!(art.get(Bytes::from_static(b"apple")), Some(Bytes::from_static(b"1")));
    assert_eq!(art.get(Bytes::from_static(b"application")), Some(Bytes::from_static(b"2")));
    assert_eq!(art.get(Bytes::from_static(b"banana")), Some(Bytes::from_static(b"3")));
    assert_eq!(art.get(Bytes::from_static(b"band")), Some(Bytes::from_static(b"4")));

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
    assert_eq!(art.get(Bytes::from_static(b"user")), Some(Bytes::from_static(b"val_user")));
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
    assert_eq!(art.get(Bytes::from_static(b"a")), Some(Bytes::from_static(b"val_a")));
    assert_eq!(art.get(Bytes::from_static(b"abc")), Some(Bytes::from_static(b"val_abc")));
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

    art.set(Bytes::from_static(b"hello_world"), Bytes::from_static(b"val"));

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

    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_data"));
    art.set(Bytes::from_static(b"user:bob"), Bytes::from_static(b"bob_data"));
    art.set(Bytes::from_static(b"user:charlie"), Bytes::from_static(b"charlie_data"));
    art.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"post_1"));

    let results = art.getn(Bytes::from_static(b"user:"));

    assert_eq!(results.len(), 3);
    assert!(results.contains(&(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_data"))));
    assert!(results.contains(&(Bytes::from_static(b"user:bob"), Bytes::from_static(b"bob_data"))));
    assert!(results.contains(&(Bytes::from_static(b"user:charlie"), Bytes::from_static(b"charlie_data"))));
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

    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"data"));

    let results = art.getn(Bytes::from_static(b"post:"));

    assert!(results.is_empty());
}

#[test]
fn test_getn_exact_key() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"));
    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_val"));

    // Préfixe exact "user" doit retourner "user" et "user:alice"
    let results = art.getn(Bytes::from_static(b"user"));

    assert_eq!(results.len(), 2);
    assert!(results.contains(&(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"))));
    assert!(results.contains(&(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_val"))));
}

#[test]
fn test_getn_prefix_in_compression() {
    // Test quand le préfixe se termine au milieu d'une compression
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"application"), Bytes::from_static(b"app_val"));

    // "app" est un préfixe de "application"
    let results = art.getn(Bytes::from_static(b"app"));

    assert_eq!(results.len(), 1);
    assert!(results.contains(&(Bytes::from_static(b"application"), Bytes::from_static(b"app_val"))));
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

    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_data"));
    art.set(Bytes::from_static(b"user:bob"), Bytes::from_static(b"bob_data"));
    art.set(Bytes::from_static(b"user:charlie"), Bytes::from_static(b"charlie_data"));
    art.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"post_1"));

    let deleted = art.deln(Bytes::from_static(b"user:"));

    assert_eq!(deleted, 3);
    assert_eq!(art.get(Bytes::from_static(b"user:alice")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:bob")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:charlie")), None);
    // post:1 doit toujours exister
    assert_eq!(art.get(Bytes::from_static(b"post:1")), Some(Bytes::from_static(b"post_1")));
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

    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"data"));

    let deleted = art.deln(Bytes::from_static(b"post:"));

    assert_eq!(deleted, 0);
    // user:alice doit toujours exister
    assert_eq!(art.get(Bytes::from_static(b"user:alice")), Some(Bytes::from_static(b"data")));
}

#[test]
fn test_deln_exact_key_with_children() {
    let mut art = OxidArt::new();

    art.set(Bytes::from_static(b"user"), Bytes::from_static(b"user_val"));
    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"alice_val"));
    art.set(Bytes::from_static(b"user:bob"), Bytes::from_static(b"bob_val"));

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

    art.set(Bytes::from_static(b"application"), Bytes::from_static(b"app_val"));
    art.set(Bytes::from_static(b"apple"), Bytes::from_static(b"apple_val"));

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
    assert_eq!(art.get(Bytes::from_static(b"a")), Some(Bytes::from_static(b"1")));
    assert_eq!(art.get(Bytes::from_static(b"ab")), None);
    assert_eq!(art.get(Bytes::from_static(b"abc")), None);
    assert_eq!(art.get(Bytes::from_static(b"b")), Some(Bytes::from_static(b"6")));
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

    art.set(Bytes::from_static(b"user:alice"), Bytes::from_static(b"old"));
    art.deln(Bytes::from_static(b"user:"));

    // Réinsérer après suppression
    art.set(Bytes::from_static(b"user:bob"), Bytes::from_static(b"new"));

    assert_eq!(art.get(Bytes::from_static(b"user:alice")), None);
    assert_eq!(art.get(Bytes::from_static(b"user:bob")), Some(Bytes::from_static(b"new")));
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
    assert_eq!(art.get(Bytes::from_static(b"world")), Some(Bytes::from_static(b"3")));
}

// ============ Tests avec dictionnaire français ============

#[test]
fn test_french_words_insert_and_deln_all() {
    let mut art = OxidArt::new();

    let words: Vec<&str> = FRENCH_WORDS.lines().collect();
    let word_count = words.len();

    // Insérer tous les mots
    for word in &words {
        art.set(Bytes::from(*word), Bytes::from(*word));
    }

    // Vérifier quelques mots au hasard
    assert!(art.get(Bytes::from_static(b"bonjour")).is_some());
    assert!(art.get(Bytes::from_static(b"ordinateur")).is_some());

    // Supprimer tout avec préfixe vide
    let deleted = art.deln(Bytes::from_static(b""));

    assert_eq!(deleted, word_count);

    // Vérifier que la slab map ne contient plus que le root (1 node)
    assert_eq!(art.map.len(), 1);

    // Note: child_list peut contenir 1 entry (huge_childs de root, non libéré pour simplifier)
    assert!(art.child_list.len() <= 1);
}

#[test]
fn test_french_words_deln_prefix_a() {
    let mut art = OxidArt::new();

    let words: Vec<&str> = FRENCH_WORDS.lines().collect();
    let total_count = words.len();

    // Compter les mots qui commencent par 'a'
    let words_starting_with_a = words.iter().filter(|w| w.starts_with('a')).count();
    let words_not_starting_with_a = total_count - words_starting_with_a;

    // Insérer tous les mots
    for word in &words {
        art.set(Bytes::from(*word), Bytes::from(*word));
    }

    // Supprimer tous les mots commençant par 'a'
    let deleted = art.deln(Bytes::from_static(b"a"));

    assert_eq!(deleted, words_starting_with_a);

    // Vérifier qu'un mot en 'a' n'existe plus
    assert_eq!(art.get(Bytes::from_static(b"abricot")), None);
    assert_eq!(art.get(Bytes::from_static(b"amour")), None);

    // Vérifier qu'un mot ne commençant pas par 'a' existe toujours
    assert!(art.get(Bytes::from_static(b"bonjour")).is_some());
    assert!(art.get(Bytes::from_static(b"maison")).is_some());

    // Vérifier avec getn que les mots restants sont corrects
    let remaining = art.getn(Bytes::from_static(b""));
    assert_eq!(remaining.len(), words_not_starting_with_a);
}

#[test]
fn test_french_words_get_all_after_insert() {
    let mut art = OxidArt::new();

    let words: Vec<&str> = FRENCH_WORDS.lines().collect();

    // Insérer tous les mots
    for word in &words {
        art.set(Bytes::from(*word), Bytes::from(*word));
    }

    // Vérifier que tous les mots sont accessibles
    for word in &words {
        let result = art.get(Bytes::from(*word));
        assert!(result.is_some(), "Word '{}' not found", word);
        assert_eq!(result.unwrap(), Bytes::from(*word));
    }
}
