use parking_lot::RwLock;

#[derive(Debug)]
pub struct ConcurrentMap<const N: usize, V>(RwLock<ConcurrentMapInternal<N, V>>);

#[derive(Debug)]
enum ConcurrentMapInternal<const N: usize, V>{
    Item(Box<([u8; N], V)>),
    List(Box<[ConcurrentMap<N, V>; 4]>),
    Empty
}

impl<const N: usize, V: Copy> ConcurrentMapInternal<N, V> {
    fn new_item(key: [u8; N], value: V) -> Self{
        Self::Item(Box::new((key, value)))
    }
}

impl<const N: usize, V: Copy> ConcurrentMap<N, V>{

    pub fn get_used_percent(&self) -> f64{
        (((size_of::<[u8; N]>() + size_of::<V>()) * self.len()) as f64) / (self.get_memory_size() as f64)
    }

    pub fn get_memory_size(&self) -> usize{
        size_of::<Self>() +
            match &*self.0.read() {
                ConcurrentMapInternal::Item(_) => size_of::<[u8; N]>() + size_of::<V>(),
                ConcurrentMapInternal::List(list) => list.iter().map(|x| x.get_memory_size()).sum(),
                ConcurrentMapInternal::Empty => 0
            }
    }

    const fn get_index(key: [u8; N], depth: usize) -> usize{
        (match depth % 4{
            0 => (key[depth/4] & 0b11000000) >> 6,
            1 => (key[depth/4] & 0b00110000) >> 4,
            2 => (key[depth/4] & 0b00001100) >> 2,
            _ => key[depth/4] & 0b00000011
        }) as usize
    }

    pub fn is_empty(&self) -> bool{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(_) => false,
            ConcurrentMapInternal::List(list) => list.iter().all(|x| x.is_empty()),
            ConcurrentMapInternal::Empty => true
        }
    }

    pub fn depth(&self) -> usize{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(_) => 1,
            ConcurrentMapInternal::List(list) => 1 + list.iter().map(|x| x.depth()).max().unwrap(),
            ConcurrentMapInternal::Empty => 0
        }
    }

    pub fn len(&self) -> usize{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(_) => 1,
            ConcurrentMapInternal::List(list) => list.iter().map(|x| x.len()).sum(),
            ConcurrentMapInternal::Empty => 0
        }
    }

    pub const fn new() -> Self{
        Self(RwLock::new(ConcurrentMapInternal::Empty))
    }

    pub fn clear(&self){
        *self.0.write() = ConcurrentMapInternal::Empty;
    }

    pub fn get(&self, key: [u8; N]) -> Option<V>{
        self.get_internal(key, 0)
    }

    fn get_internal(&self, key: [u8; N], depth: usize) -> Option<V>{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(item) => if item.0 == key {Some(item.1)} else {None},
            ConcurrentMapInternal::List(list) => list[Self::get_index(key, depth)].get_internal(key, depth + 1),
            ConcurrentMapInternal::Empty => None
        }
    }

    /*pub fn get_or_closest_by_key(&self, key: [u8; N], include_key: bool) -> Option<([u8; N], V)>{
        self.get_or_closest_by_key_internal(key, include_key, 0, None).map(|x| x.0)
    }

    fn get_or_closest_by_key_internal(&self, key: [u8; N], include_key: bool, depth: usize, closest: Option<([u8; N], V)>) -> Option<(([u8; N], V), bool, bool)>{
        //go down to where key would be
        //if key is there and include key is true, return
        //if above is false then go up and down the right hand side of left index in list and left hand side of right index
        //if no left or right index then need to go up until there is one
        self.0.read().map(|read_lock| {
            match &*read_lock{
                None => None,
                Some(inner) => {
                    match inner{
                        ConcurrentMapInternal::Item(item_key_value) => {
                            if (item_key_value.0 != key) || include_key {Some(item_key_value)} else {None}
                        }
                        ConcurrentMapInternal::List(list) => {
                            let index = Self::get_index(key, depth);
                            let result = list[index].get_or_closest_by_key_internal(key, include_key, depth + 1, closest);
                            
                        }
                    }
                }
            }
        }).unwrap();
    }

    const HALF_POINT: [u8; N] = {
        let mut array = [0; N];
        array[0] = 1;
        array
    };

    fn get_abs_diff(item_1: [u8; N], item_2: [u8; N]) -> [u8; N]{
        let inner_function = |item_1_inner: [u8; N], item_2_inner: [u8; N]| {
            let mut result = [0; N];
            let mut borrow = 0;
            for i in (0..N).rev() {
                if item_1_inner[i] > item_2_inner[i]{
                    result[i] = item_1_inner[i] - item_2_inner[i] - borrow;
                    borrow = 0;
                }
                else if item_1_inner[i] == item_2_inner[i]{
                    if borrow == 1 {result[i] = u8::MAX}
                    else {result[i] = 0}
                }
                else{
                    result[i] = u8::MAX - (item_2_inner[i] - item_1_inner[i]) + 1 - borrow;
                    borrow = 1;
                }
            }
            result
        };
        let diff = inner_function(item_1, item_2);
        if diff > Self::HALF_POINT {inner_function(item_2, item_1)} else {diff}
    }*/

    pub fn get_min(&self) -> Option<([u8; N], V)>{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(item_key_value) => Some((item_key_value.0, item_key_value.1)),
            ConcurrentMapInternal::List(list) => list.iter().find_map(|x| x.get_min()),
            ConcurrentMapInternal::Empty => None
        }
    }

    pub fn get_max(&self) -> Option<([u8; N], V)>{
        match &*self.0.read(){
            ConcurrentMapInternal::Item(item_key_value) => Some((item_key_value.0, item_key_value.1)),
            ConcurrentMapInternal::List(list) => list.iter().rev().find_map(|x| x.get_max()),
            ConcurrentMapInternal::Empty => None
        }
    }

    pub fn insert_or_update(&self, key: [u8; N], value: V) -> bool{
        self.insert_or_update_if(key, value, &|_,_| true)
    }

    pub fn insert_or_update_if(&self, key: [u8; N], value: V, should_update: &impl Fn(&V, &V) -> bool) -> bool{
        self.insert_or_update_if_internal(key, value, should_update, 0)
    }

    fn insert_or_update_if_internal(&self, key: [u8; N], value: V, should_update: &impl Fn(&V, &V) -> bool, depth: usize) -> bool{
        loop{
            match &*self.0.read(){
                ConcurrentMapInternal::Item(_) => (), //change to write_lock
                ConcurrentMapInternal::List(list) => return list[Self::get_index(key, depth)].insert_or_update_if_internal(key, value, should_update, depth + 1),
                ConcurrentMapInternal::Empty => () //change to write lock
            }
            let mut write_lock = self.0.write();
            match &mut *write_lock{
                ConcurrentMapInternal::Item(item_key_value) => {
                    return if item_key_value.0 == key{
                        //update
                        if should_update(&item_key_value.1, &value){
                            item_key_value.1 = value;
                            true
                        }
                        else {false}
                    }
                    else{
                        //insert and restructure
                        *write_lock = Self::deepen_tree((item_key_value.0, item_key_value.1), (key, value), depth);
                        true
                    }
                }
                ConcurrentMapInternal::List(_) => (), //change back to read lock
                ConcurrentMapInternal::Empty => {
                    *write_lock = ConcurrentMapInternal::new_item(key, value);
                    return true;
                }
            }
        }
    }

    fn deepen_tree(item_1: ([u8; N], V), item_2: ([u8; N], V), depth: usize) -> ConcurrentMapInternal<N, V> {
        let item_1_index = Self::get_index(item_1.0, depth);
        let item_2_index = Self::get_index(item_2.0, depth);
        let new_list = [const {Self::new()}; 4];
        if item_1_index == item_2_index {
            *new_list[item_1_index].0.write() = Self::deepen_tree(item_1, item_2, depth + 1);
        }
        else{
            *new_list[item_1_index].0.write() = ConcurrentMapInternal::new_item(item_1.0, item_1.1);
            *new_list[item_2_index].0.write() = ConcurrentMapInternal::new_item(item_2.0, item_2.1);
        }
        ConcurrentMapInternal::List(Box::new(new_list))
    }

    pub fn remove(&self, key: [u8; N]){
        self.remove_if(key, &|_| true);
    }

    pub fn remove_if(&self, key: [u8; N], should_remove: &impl Fn(&V) -> bool){
        self.remove_if_internal(key, should_remove, 0);
    }

    fn remove_if_internal(&self, key: [u8; N], should_remove: &impl Fn(&V) -> bool, depth: usize){
        //this currently removes the item but does not prune the lists back to where it could be
        //add this ability in when figure out how to, using read and write locks to be more space efficient
        loop{
            match &*self.0.read(){
                ConcurrentMapInternal::Item(_) => (), //go to write lock
                ConcurrentMapInternal::List(list) => {
                    list[Self::get_index(key, depth)].remove_if_internal(key, should_remove, depth + 1);
                    return
                }
                ConcurrentMapInternal::Empty => return
            }
            let mut write_lock = self.0.write();
            match &mut *write_lock{
                ConcurrentMapInternal::Item(item_key_value) => {
                    if (item_key_value.0 == key) && should_remove(&item_key_value.1) {
                        *write_lock = ConcurrentMapInternal::Empty;
                    }
                    return
                }
                ConcurrentMapInternal::List(_) => (), //change back to read lock
                ConcurrentMapInternal::Empty => return
            }
        }
    }
}