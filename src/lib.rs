use std::{hash::{DefaultHasher, Hash, Hasher}, sync::{Mutex, RwLock}};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
struct ConcurrentBSTNode<K,V>{
    key: K,
    value: V,
    child_nodes: [Option<(bool, K)>; 2]
}

#[derive(Debug)]
struct ConcurrentBSTInternal<K,V>{
    no_elements: Mutex<usize>,
    list: Vec<Mutex<Option<ConcurrentBSTNode<K,V>>>>
}

#[derive(Debug)]
pub struct ConcurrentBST<K,V>{
    inner: RwLock<ConcurrentBSTInternal<K,V>>,
    root_node_key: Mutex<Option<K>>
}

pub trait ShouldUpdate{
    fn should_update_to(&self, other: &Self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
enum InsertStatus{
    Updated(bool),
    Inserted,
    RebaseRequired
}

impl<K: Copy + Ord + Eq + Hash, V: ShouldUpdate + Copy> ConcurrentBST<K,V>{

    pub fn new() -> Self{
        Self{
            inner: RwLock::new(ConcurrentBSTInternal{
                no_elements: Mutex::new(0),
                list: Vec::from([const {Mutex::new(None)}; 1024])
            }),
            root_node_key: Mutex::new(None)
        }
    }

    fn get_key_hash(key: K, max_value: usize) -> usize{
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % (max_value as u64)) as usize
    }

    pub fn add_or_update(&self, key: K, value: V) -> bool{
        let inserted = self.inner.read().map(|rw_lock| {
            let inner_function = |start_key| {
                let list_length = rw_lock.list.len();
                let mut inserted = None;
                let mut current_key = start_key;
                let mut current_key_hash = (current_key, Self::get_key_hash(current_key, list_length));
                let mut break_while_loop;
                let mut counter;
                loop {
                    match inserted {
                        Some(result) => return result,
                        None => {
                            if current_key_hash.0 != current_key{
                                current_key_hash = (current_key,  Self::get_key_hash(current_key, list_length));
                            }
                            break_while_loop = false;
                            counter = 0;
                            while !break_while_loop && inserted.is_none() {
                                rw_lock.list[(current_key_hash.1 + counter) % list_length].lock().map(|mut mutex_lock| {
                                    match *mutex_lock {
                                        None => {
                                            rw_lock.no_elements.lock().map(|mut no_elements| {
                                                if *no_elements >= (list_length / 2){
                                                    inserted = Some(InsertStatus::RebaseRequired);
                                                }
                                                else{
                                                    *mutex_lock = Some(ConcurrentBSTNode {
                                                        key,
                                                        value,
                                                        child_nodes: [None; 2]
                                                    });
                                                    inserted = Some(InsertStatus::Inserted);
                                                    *no_elements += 1;
                                                }
                                            }).unwrap();
                                        }
                                        Some(mut node) => {
                                            if node.key == current_key {
                                                if current_key == key{
                                                    inserted = Some(InsertStatus::Updated(
                                                        if node.value.should_update_to(&value){
                                                            node.value = value;
                                                            true
                                                        }
                                                        else {false}
                                                    ));
                                                }
                                                else{
                                                    match *node.child_nodes[if key < current_key {0} else {1}].get_or_insert((false, key)){
                                                        (true, _) => (),
                                                        (false, child_key) => {
                                                            break_while_loop = true;
                                                            current_key = child_key;
                                                        }
                                                    }
                                                }
                                            }
                                            else {counter += 1}
                                        }
                                    }
                                }).unwrap();
                            }
                        }
                    }
                }
            };

            let mut inner_function_result = None;
            let root_node_key = *self.root_node_key.lock().unwrap().get_or_insert_with(|| {
                inner_function_result = Some(inner_function(key));
                key
            });

            inner_function_result.unwrap_or(inner_function(root_node_key))

        }).unwrap();

        match inserted{
            InsertStatus::Updated(was_updated) => was_updated,
            InsertStatus::Inserted => true,
            InsertStatus::RebaseRequired => {
                //rebase the vec
                self.inner.write().map(|mut rw_lock| {
                    if *rw_lock.no_elements.lock().unwrap() >= (rw_lock.list.len() / 2) {
                        let mut new_vec = Vec::<Mutex<Option<ConcurrentBSTNode<K, V>>>>::new();
                        for _ in 0..(rw_lock.list.len() * 2) { new_vec.push(Mutex::new(None)) }
                        let mut key_hash = 0;
                        let mut counter = 0;
                        for possible_node in rw_lock.list.iter() {
                            possible_node.lock().map(|mutex_lock| {
                                match *mutex_lock {
                                    None => (),
                                    Some(node) => {
                                        key_hash = Self::get_key_hash(node.key, new_vec.len());
                                        counter = 0;
                                        while new_vec[(key_hash + counter) % new_vec.len()].lock().unwrap().is_some() {counter += 1}
                                        *new_vec[(key_hash + counter) % new_vec.len()].lock().unwrap() = Some(node);
                                    }
                                }
                            }).unwrap();
                        }
                        rw_lock.list = new_vec;
                    }
                }).unwrap();

                self.add_or_update(key, value)
            }
        }
    }

    pub fn remove(&self, key: K){

    }

    pub fn remove_if(&self, key: K, should_remove: impl FnOnce(&V) -> bool) -> bool{
        true
    }

}