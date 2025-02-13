use std::fmt;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub struct MaxDepthReachedError;

impl fmt::Display for MaxDepthReachedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Insert would exceed max depth given")
    }
}

impl std::error::Error for MaxDepthReachedError {}

#[derive(Debug)]
struct ConcurrentBSTInternal<const N: usize, V, const MAX_DEPTH: u16>{
    key: [u8; N],
    value: V,
    child_nodes: [ConcurrentBSTMap<N, V, MAX_DEPTH>; 2]
}

impl<const N: usize, V: Copy, const MAX_DEPTH: u16> ConcurrentBSTInternal<N, V, MAX_DEPTH>{

    fn new(key: [u8; N], value: V) -> Self {
        Self {
            key,
            value,
            child_nodes: [const {ConcurrentBSTMap::new()}; 2]
        }
    }
}

#[derive(Debug)]
pub struct ConcurrentBSTMap<const N: usize, V, const MAX_DEPTH: u16>(RwLock<Option<Box<ConcurrentBSTInternal<N, V, MAX_DEPTH>>>>);

impl<const N: usize, V: Copy, const MAX_DEPTH: u16> ConcurrentBSTMap<N, V, MAX_DEPTH>{
    
    pub fn clear(&self){
        *self.0.write().unwrap() = None;
    }

    pub fn contains_key(&self, key: [u8; N]) -> bool{
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

    pub fn get(&self, key: [u8; N]) -> Option<V>{
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

    pub fn get_all_key_values(&self) -> Vec<([u8; N],V)>{
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

    pub fn get_max(&self) -> Option<([u8; N],V)> {
        self.get_min_or_max(false)
    }

    pub fn get_min(&self) -> Option<([u8; N],V)>{
        self.get_min_or_max(true)
    }

    fn get_or_closest_by_key_internal(&self, key: [u8; N], closest: ([u8; N], V), include_key: bool, all_left: bool, all_right: bool) -> Option<(([u8; N], V), bool, bool)>{
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

    pub fn get_or_closest_by_key(&self, key: [u8; N], include_key: bool) -> Option<([u8; N], V)>{
        
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
                                if Self::get_abs_diff(key, result.0) < Self::get_abs_diff(key, key_value.0) {result} else {key_value}
                            }
                        }
                    }
                )
            }
        }
    }

    pub fn insert_or_update(&self, key: [u8; N], value: V, should_update: &impl Fn(&V, &V) -> bool) -> Result<bool, MaxDepthReachedError>{
        self.insert_or_update_internal(key, value, should_update, MAX_DEPTH)
    }
    
    fn insert_or_update_internal(&self, key: [u8; N], value: V, should_update: &impl Fn(&V, &V) -> bool, current_depth: u16) -> Result<bool, MaxDepthReachedError>{
        if current_depth == 0 {return Err(MaxDepthReachedError)}
        loop{
            match self.0.read().map(|read_lock| {
                match &*read_lock{
                    None => None,
                    Some(node) => {
                        if node.key != key {
                            Some(
                                //should never be 0
                                if current_depth < 2 {Err(MaxDepthReachedError)}
                                else {node.child_nodes[Self::get_index(key, node.key)].insert_or_update_internal(key, value, should_update, current_depth - 1)}
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

    pub fn remove(&self, key: [u8; N]){
        self.remove_if(key, &|_| true)
    }

    pub fn remove_if(&self, key: [u8; N], should_remove: &impl Fn(&V) -> bool){
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
                        else if should_remove(&node.value){
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

    fn get_abs_diff(item_1: [u8; N], item_2: [u8; N]) -> [u8; N]{
        let inner_function = |larger: [u8; N], smaller: [u8; N]| {
            let mut result = [0; N];
            let mut borrow = 0;
            for i in (0..N).rev() {
                let diff = (larger[i] as i16) - (smaller[i] as i16) - borrow;
                if diff < 0 {
                    borrow = 1;
                    result[i] = (diff + 256) as u8;
                } else {
                    borrow = 0;
                    result[i] = diff as u8;
                }
            }
            result
        };
        let mut result = if item_1 > item_2 {inner_function(item_1, item_2)} else {inner_function(item_2, item_1)};
        if (result[0] >> 7) == 1 {
            //other way around is shorter or equal
            //minus one
            let mut index = N-1;
            while result[index] == 0 {
                result[index] = u8::MAX;
                index -= 1;
            }
            result[index] -= 1;
            inner_function([u8::MAX; N], result)
        }
        else {result}
    }

    fn get_index(target: [u8; N], current: [u8; N]) -> usize{
        if target < current {0} else {1}
    }

    fn get_min_or_max(&self, min: bool) -> Option<([u8; N],V)>{
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => None,
                Some(node) => {
                    Some(node.child_nodes[if min {0} else {1}].get_min_or_max(min).unwrap_or((node.key, node.value)))
                }
            }
        }).unwrap()
    }

    fn get_replacement_key_value(&self, go_left: bool) -> Option<([u8; N],V)>{
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

#[derive(Debug)]
pub struct ConcurrentBSTSet<const N: usize, const MAX_DEPTH: u16>(ConcurrentBSTMap<N, (), MAX_DEPTH>);

impl<const N: usize, const MAX_DEPTH: u16> ConcurrentBSTSet<N, MAX_DEPTH>{

    pub fn clear(&self){
        self.0.clear();
    }

    pub fn contains_key(&self, key: [u8; N]) -> bool{
        self.0.contains_key(key)
    }

    pub fn depth(&self) -> u32{
        self.0.depth()
    }

    pub fn get_max(&self) -> Option<[u8; N]>{
        self.0.get_max().map(|x| x.0)
    }

    pub fn get_min(&self) -> Option<[u8; N]>{
        self.0.get_min().map(|x| x.0)
    }

    pub fn insert(&self, key: [u8; N]) -> Result<(), MaxDepthReachedError>{
        self.0.insert_or_update(key, (), &crate::NEVER_UPDATE).map(|_| ())
    }

    pub fn is_empty(&self) -> bool{
        self.0.is_empty()
    }

    pub fn len(&self) -> usize{
        self.0.len()
    }

    pub const fn new() -> Self{
        Self(ConcurrentBSTMap::new())
    }

    pub fn remove(&self, key: [u8; N]){
        self.0.remove_if(key, &|_| true)
    }
}