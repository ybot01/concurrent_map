use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, SystemTime};
use rand::distributions::{Distribution, Standard};
use rand::random;
use tokio::task::JoinHandle;
use concurrent_bst_map::{ConcurrentBSTMap, Values, ALWAYS_UPDATE, DEFAULT_MAX_DEPTH};

fn should_update<T: Ord>(value_1: &T, value_2: &T) -> bool{
    value_2 > value_1
}

fn get_vec_of_key_values<T>(length: usize) -> Vec<T> where Standard: Distribution<T>{
    let mut to_return = Vec::<T>::new();
    for _ in 0..length {to_return.push(random())}
    to_return
}

#[test]
fn length_test(){
    let expected = 10000;
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    get_vec_of_key_values::<(u64,u64)>(expected).iter()
        .for_each(|x| _ = bst.insert_or_update(x.0, x.1, &ALWAYS_UPDATE, DEFAULT_MAX_DEPTH));
    assert_eq!(bst.len(), expected);
}

#[test]
fn depth_test(){
    let expected = 10000;
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    get_vec_of_key_values::<(u64,u64)>(expected).iter()
        .for_each(|x| _ = bst.insert_or_update(x.0, x.1, &ALWAYS_UPDATE, DEFAULT_MAX_DEPTH));
    println!("{}", bst.depth());
}


#[test]
fn remove_test(){
    let expected = 10000;
    let to_insert = get_vec_of_key_values::<(u64,u64)>(expected);
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    to_insert.iter().for_each(|x| _ = bst.insert_or_update(x.0, x.1, &ALWAYS_UPDATE, DEFAULT_MAX_DEPTH));
    to_insert.iter().for_each(|x| bst.remove(x.0));
    assert!(to_insert.iter().all(|x| bst.get(x.0).is_none()));
}

#[test]
fn should_update_test() {
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    let (key, mut value) = (1000, 0);
    assert!(bst.insert_or_update(key, value, &should_update, DEFAULT_MAX_DEPTH).is_ok_and(|x| x));
    value += 1;
    assert!(bst.insert_or_update(key, value, &should_update, DEFAULT_MAX_DEPTH).is_ok_and(|x| x));
    value -= 1;
    assert!(!bst.insert_or_update(key, value, &should_update, DEFAULT_MAX_DEPTH).is_ok_and(|x| x));
}

#[test]
fn insert_and_get_test() {
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    _ = bst.insert_or_update(0, 1, &ALWAYS_UPDATE, DEFAULT_MAX_DEPTH);
    assert!(bst.get(0).is_some_and(|x| x == 1));
}

#[test]
fn bench_insert_or_update_if(){
    let bst = ConcurrentBSTMap::<u64, u64>::new();
    let (key, mut value) = (1000, 0);
    let mut true_count = 0;
    let total = 1000000;
    let start_time = SystemTime::now();
    for _ in 0..total{
        if bst.insert_or_update(key, value, &should_update, DEFAULT_MAX_DEPTH).is_ok_and(|x| x) {true_count += 1};
        value += 1;
    }
    println!("{}", total as f64 / SystemTime::now().duration_since(start_time).unwrap().as_secs_f64());
    assert_eq!(true_count, total);
}

static GLOBAL_BST: ConcurrentBSTMap<u64, u64> = ConcurrentBSTMap::new();

static TRUE_COUNT: AtomicUsize = AtomicUsize::new(0);

const NO_THREADS: usize = 10;
const TOTAL_PER_THREAD: usize = 100000;

static USER_LIST: LazyLock<RwLock<Vec<(u64, u64)>>> = LazyLock::new(|| RwLock::new(get_vec_of_key_values(NO_THREADS*TOTAL_PER_THREAD)));

#[test]
fn bench_multi_thread_insert_or_update_if_and_remove(){
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            _ = USER_LIST.read().unwrap().clone();
            let mut threads= Vec::<JoinHandle<Duration>>::new();
            for i in 0..NO_THREADS{
                threads.push(tokio::spawn(async move{
                    let start_index = TOTAL_PER_THREAD * i;
                    let start_time = SystemTime::now();
                    USER_LIST.read().map(|read_lock| {
                        for i in start_index..(start_index+TOTAL_PER_THREAD) {
                            let user = read_lock[i];
                            if GLOBAL_BST.insert_or_update(user.0, user.1, &should_update, DEFAULT_MAX_DEPTH).is_ok_and(|x| x){
                                TRUE_COUNT.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }).unwrap();
                    SystemTime::now().duration_since(start_time).unwrap()
                }))
            }
            while threads.iter().any(|x| !x.is_finished()) {}
            let mut max_duration = Duration::ZERO;
            let mut duration;
            for i in threads{
                duration = i.await.unwrap();
                if duration > max_duration{
                    max_duration = duration;
                }
            }
            println!("{}", (NO_THREADS*TOTAL_PER_THREAD) as f64 / max_duration.as_secs_f64());
            assert_eq!(TRUE_COUNT.load(Ordering::Relaxed), NO_THREADS*TOTAL_PER_THREAD);
            
            threads = Vec::new();
            for i in 0..NO_THREADS{
                threads.push(tokio::spawn(async move{
                    let start_index = TOTAL_PER_THREAD * i;
                    let start_time = SystemTime::now();
                    USER_LIST.read().map(|read_lock| {
                        for i in start_index..(start_index+TOTAL_PER_THREAD) {
                            GLOBAL_BST.remove(read_lock[i].0);
                        }
                    }).unwrap();
                    SystemTime::now().duration_since(start_time).unwrap()
                }))
            }
            while threads.iter().any(|x| !x.is_finished()) {}
            max_duration = Duration::ZERO;
            for i in threads{
                duration = i.await.unwrap();
                if duration > max_duration{
                    max_duration = duration;
                }
            }
            println!("{}", (NO_THREADS*TOTAL_PER_THREAD) as f64 / max_duration.as_secs_f64());
        });
}