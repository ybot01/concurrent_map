use std::fmt;
use std::ops::Sub;
use std::sync::RwLock;
use crate::{ALWAYS_UPDATE, NEVER_UPDATE};

pub trait Constants {
    const MAX_DEPTH: u32;
    const MAX: Self;
    const ONE: Self;
    const ZERO: Self;
}

#[derive(Debug, Clone, Copy)]
pub struct MaxDepthReachedError;

impl fmt::Display for MaxDepthReachedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Insert would exceed max depth given")
    }
}

impl std::error::Error for MaxDepthReachedError {}

#[derive(Debug)]
struct ConcurrentBSTInternal<K,V>{
    key: K,
    value: V,
    child_nodes: [ConcurrentBSTMap<K,V>; 2]
}

impl<K: Copy + Ord + Sub<Output = K> + Constants, V: Copy> ConcurrentBSTInternal<K,V>{

    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            child_nodes: [const { ConcurrentBSTMap::new() }; 2]
        }
    }
}

#[derive(Debug)]
pub struct ConcurrentBSTMap<K,V>(RwLock<Option<Box<ConcurrentBSTInternal<K,V>>>>);

impl<K: Copy + Ord + Sub<Output = K> + Constants, V: Copy> ConcurrentBSTMap<K,V>{

    pub fn clear(&self){
        *self.0.write().unwrap() = None;
    }

    pub fn contains_key(&self, key: K) -> bool{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => false,
                Some(node) => {
                    if node.key == key {true}
                    else {node.child_nodes[Self::get_index(key, node.key)].contains_key(key)}
                }
            }
        }).unwrap()
    }

    pub fn copy(&self, rand_index_func: impl Fn(usize) -> usize) -> Self{
        let mut all_key_values = self.get_all_key_values();
        loop{
            let mut error = false;
            let new_bst = ConcurrentBSTMap::new();
            let (mut key, mut value);
            while (all_key_values.len() > 0) && !error{
                (key, value) = all_key_values.swap_remove(rand_index_func(all_key_values.len()));
                if new_bst.insert_or_update(key, value, &NEVER_UPDATE).is_err() {error = true}
            }
            if !error {return new_bst}
        }
    }

    pub fn depth(&self) -> u32{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => 0,
                Some(node) => {
                    1 + node.child_nodes[0].depth().max(node.child_nodes[1].depth())
                }
            }
        }).unwrap()
    }

    pub fn get(&self, key: K) -> Option<V>{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => None,
                Some(node) => {
                    if node.key == key {Some(node.value)}
                    else {node.child_nodes[Self::get_index(key, node.key)].get(key)}
                }
            }
        }).unwrap()
    }

    pub fn get_all_key_values(&self) -> Vec<(K,V)>{
        self.0.read().map(|read_lock| {
            match &*read_lock {
                None => Vec::new(),
                Some(node) => {
                    let mut all = node.child_nodes[0].get_all_key_values();
                    all.extend(node.child_nodes[1].get_all_key_values());
                    all.push((node.key, node.value));
                    all
                }
            }
        }).unwrap()
    }

    pub fn get_max(&self) -> Option<(K,V)> {
        self.get_min_or_max(false)
    }

    pub fn get_min(&self) -> Option<(K,V)>{
        self.get_min_or_max(true)
    }

    fn get_or_closest_by_key_internal(&self, key: K, closest: (K, V), include_key: bool, all_left: bool, all_right: bool) -> Option<((K, V), bool, bool)>{
        self.0.read().map(|read_lock| {
            match &*read_lock {
                None => None,
                Some(node) => {
                    let new_closest = if ((node.key != key) || include_key) && (Self::get_abs_diff(key, node.key) < Self::get_abs_diff(key, closest.0)) {(node.key, node.value)} else {closest};
                    let index = Self::get_index(key, node.key);
                    Some(
                        node.child_nodes[index].get_or_closest_by_key_internal(
                            key,
                            new_closest,
                            include_key,
                            (index == 0) && all_left,
                            (index == 1) && all_right
                        ).unwrap_or((
                            new_closest,
                            all_left && node.child_nodes[0].0.read().unwrap().is_none(),
                            all_right && node.child_nodes[1].0.read().unwrap().is_none()
                        ))
                    )
                }
            }
        }).unwrap()
    }

    pub fn get_or_closest_by_key(&self, key: K, include_key: bool) -> Option<(K, V)>{

        let get_loop_around_diff = |item_1, item_2| {
            let abs_diff = Self::get_abs_diff(item_1, item_2);
            //should never be 0
            if abs_diff == K::ZERO {K::ZERO}
            else {K::MAX - (abs_diff - K::ONE)}
        };

        match self.0.read().map(|read_lock| {
            match &*read_lock {
                None => None,
                Some(node) => {
                    let key_value = (node.key, node.value);
                    let index = Self::get_index(key, node.key);
                    Some(
                        node.child_nodes[Self::get_index(key, node.key)].get_or_closest_by_key_internal(
                            key,
                            key_value,
                            include_key,
                            index == 0,
                            index == 1
                        ).unwrap_or((key_value, false, false))
                    )
                }
            }
        }).unwrap(){
            None => None,
            Some((key_value, is_min, is_max)) => {
                Some(
                    if key_value.0 == key {key_value}
                    else{
                        match if is_min {self.get_max()} else if is_max {self.get_min()} else {None}{
                            None => key_value,
                            Some(result) => {
                                if get_loop_around_diff(key, result.0) < get_loop_around_diff(key, key_value.0) {result} else {key_value}
                            }
                        }
                    }
                )
            }
        }
    }

    pub fn get_next(&self, key: K) -> Option<(K, V)>{
        self.0.read().map(|read_lock| {
            match &*read_lock {
                None => None,
                Some(node) => {
                    [
                        if node.key > key {Some((node.key, node.value))} else {None},
                        node.child_nodes[Self::get_index(key, node.key)].get_next(key)
                    ].iter().filter_map(|x| *x)
                        .min_by_key(|x| x.0 - key)
                }
            }
        }).unwrap()
    }

    pub fn insert_or_update(&self, key: K, value: V, should_update: &impl Fn(&V, &V) -> bool) -> Result<bool, MaxDepthReachedError>{
        self.insert_or_update_internal(key, value, should_update, K::MAX_DEPTH)
    }

    fn insert_or_update_internal(&self, key: K, value: V, should_update: &impl Fn(&V, &V) -> bool, depth_remaining: u32) -> Result<bool, MaxDepthReachedError>{
        if depth_remaining == 0 {return Err(MaxDepthReachedError)}
        loop{
            match self.0.read().map(|read_lock| {
                match &*read_lock{
                    None => None,
                    Some(node) => {
                        if node.key != key {
                            Some(
                                //should never be 0
                                if depth_remaining < 2 {Err(MaxDepthReachedError)}
                                else {node.child_nodes[Self::get_index(key, node.key)].insert_or_update_internal(key, value, should_update, depth_remaining - 1)}
                            )
                        }
                        else {None}
                    }
                }
            }).unwrap(){
                None => (),
                Some(result) => return result
            }
            match self.0.write().map(|mut write_lock| {
                match &mut *write_lock{
                    None => {
                        //insert
                        *write_lock = Some(Box::new(ConcurrentBSTInternal::new(key, value)));
                        Some(Ok(true))
                    }
                    Some(node) => {
                        //if a different key than before then retry the read lock
                        if node.key != key {None}
                        else{
                            //update
                            Some(Ok(
                                if should_update(&node.value, &value){
                                    node.value = value;
                                    true
                                }
                                else {false}
                            ))
                        }
                    }
                }
            }).unwrap(){
                None => (),
                Some(result) => return result
            }
        }
    }

    pub fn is_empty(&self) -> bool{
        self.len() == 0
    }

    pub fn iter(&self) -> ConcurrentBSTMapIterator<K, V>{
        ConcurrentBSTMapIterator{
            map: self,
            current_key: self.get_min().map(|x| x.0)
        }
    }

    pub fn len(&self) -> usize{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => 0,
                Some(node) => {
                    1 + node.child_nodes[0].len() + node.child_nodes[1].len()
                }
            }
        }).unwrap()
    }

    pub const fn new() -> Self{
        Self(RwLock::new(None))
    }

    pub fn remove(&self, key: K){
        self.remove_if(key, &|_, _| true)
    }

    pub fn remove_if(&self, key: K, should_remove: &impl Fn(&K, &V) -> bool){
        loop{
            if self.0.read().map(|read_lock| {
                match &*read_lock{
                    None => true,
                    Some(node) => {
                        if node.key != key {
                            node.child_nodes[Self::get_index(key, node.key)].remove_if(key, should_remove);
                            true
                        }
                        else {false}
                    }
                }
            }).unwrap() {return}
            if self.0.write().map(|mut write_lock| {
                match &mut *write_lock{
                    None => true,
                    Some(node) => {
                        //if a different key than before then retry the read lock
                        if node.key != key {false}
                        else if should_remove(&node.key, &node.value){
                            match node.child_nodes[1].get_replacement_key_value(true)
                                .or(node.child_nodes[0].get_replacement_key_value(false)) {
                                None => *write_lock = None,
                                Some(result) => (node.key, node.value) = result
                            }
                            true
                        }
                        else {true}
                    }
                }
            }).unwrap() {return}
        }
    }

    pub fn retain(&self, criteria: &impl Fn(&K, &V) -> bool){
        self.iter().for_each(|(key, value)| {
            if criteria(&key, &value){
                //delete
            }
        });
    }

    fn get_abs_diff(item_1: K, item_2: K) -> K{
        if item_2 > item_1 {item_2 - item_1} else {item_1 - item_2}
    }

    fn get_index(target: K, current: K) -> usize{
        if target < current {0} else {1}
    }

    fn get_min_or_max(&self, min: bool) -> Option<(K,V)>{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => None,
                Some(node) => {
                    Some(node.child_nodes[if min {0} else {1}].get_min_or_max(min).unwrap_or((node.key, node.value)))
                }
            }
        }).unwrap()
    }

    fn get_replacement_key_value(&self, go_left: bool) -> Option<(K,V)>{
        self.0.write().map(|mut write_lock| {
            match &mut *write_lock {
                None => None,
                Some(node) => {
                    match node.child_nodes[if go_left {0} else {1}].get_replacement_key_value(go_left){
                        None => {
                            //found replacement node with no node in chosen direction
                            let key_value = (node.key, node.value);
                            //if got opposite direction node, recursively run on that
                            match node.child_nodes[if go_left {1} else {0}].get_replacement_key_value(go_left){
                                None => *write_lock = None,
                                Some(result) => (node.key, node.value) = result
                            }
                            Some(key_value)
                        }
                        Some(result) => Some(result)
                    }
                }
            }
        }).unwrap()
    }
}

impl<K: Copy + Ord + Sub<Output = K> + Constants, V: Copy> IntoIterator for ConcurrentBSTMap<K, V>{
    type Item = (K, V);

    type IntoIter = ConcurrentBSTMapIntoIterator<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        ConcurrentBSTMapIntoIterator{
            current_key: self.get_min().map(|x| x.0),
            map: self
        }
    }
}

pub struct ConcurrentBSTMapIntoIterator<K, V> {
    map: ConcurrentBSTMap<K,V>,
    current_key: Option<K>
}

impl<K: Copy + Ord + Sub<Output = K> + Constants, V: Copy> Iterator for ConcurrentBSTMapIntoIterator<K, V>{
    type Item = (K,V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_key{
            None => None,
            Some(current_key) => {
                let next_key_value = self.map.get_next(current_key);
                self.current_key = next_key_value.map(|x| x.0);
                next_key_value
            }
        }
    }
}

pub struct ConcurrentBSTMapIterator<'a, K, V> {
    map: &'a ConcurrentBSTMap<K,V>,
    current_key: Option<K>
}

impl<'a, K: Copy + Ord + Sub<Output = K> + Constants, V: Copy> Iterator for ConcurrentBSTMapIterator<'a, K, V>{
    type Item = (K,V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_key{
            None => None,
            Some(current_key) => {
                let next_key_value = self.map.get_next(current_key);
                self.current_key = next_key_value.map(|x| x.0);
                next_key_value
            }
        }
    }
}

#[derive(Debug)]
pub struct ConcurrentBSTSet<K>(ConcurrentBSTMap<K, ()>);

impl<K: Copy + Ord + Sub<Output = K> + Constants> ConcurrentBSTSet<K>{

    pub fn clear(&self){
        self.0.clear();
    }

    pub fn contains_key(&self, key: K) -> bool{
        self.0.contains_key(key)
    }

    pub fn depth(&self) -> u32{
        self.0.depth()
    }

    pub fn get_max(&self) -> Option<K>{
        self.0.get_max().map(|x| x.0)
    }

    pub fn get_min(&self) -> Option<K>{
        self.0.get_min().map(|x| x.0)
    }

    pub fn get_next(&self, key: K) -> Option<K>{
        self.0.get_next(key).map(|x| x.0)
    }

    pub fn insert(&self, key: K) -> Result<(), MaxDepthReachedError>{
        self.0.insert_or_update(key, (), &ALWAYS_UPDATE).map(|_| ())
    }

    pub fn is_empty(&self) -> bool{
        self.0.is_empty()
    }

    pub fn iter(&self) -> ConcurrentBSTMapIterator<K, ()>{
        self.0.iter()
    }

    pub fn into_iter(self) -> ConcurrentBSTMapIntoIterator<K, ()>{
        self.0.into_iter()
    }

    pub fn len(&self) -> usize{
        self.0.len()
    }

    pub const fn new() -> Self{
        Self(ConcurrentBSTMap::new())
    }

    pub fn remove(&self, key: K){
        self.remove_if(key, &|_| true)
    }

    pub fn remove_if(&self, key: K, should_remove: &impl Fn(&K) -> bool){
        self.0.remove_if(key, &|x, _| should_remove(x))
    }

    pub fn retain(&self, criteria: &impl Fn(&K) -> bool){
        self.0.retain(&|x, _| criteria(x))
    }
}