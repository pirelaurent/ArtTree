
use std;
use std::{mem, ptr};
use std::marker::PhantomData;

use {ArtKey};

pub const MAX_PREFIX_LEN: usize = 10;
pub const SMALL_STRUCT: usize = 8;
const EMPTY_CELL: u8 = 0;

macro_rules! make_array {
    ($n:expr, $constructor:expr) => {{
        let mut items: [_; $n] = std::mem::uninitialized();
        for place in items.iter_mut() {
            std::ptr::write(place, $constructor);
        }
        items
    }}
}

type Small = [u8; SMALL_STRUCT];

#[derive(Debug)]
pub struct SmallStruct<T> {
    storage: Small,
    marker: PhantomData<T>,
}

impl<T> SmallStruct<T> {
    pub fn new(elem: T) -> Self {
        unsafe {
            let mut ret = SmallStruct { storage: mem::uninitialized(), marker: PhantomData };
            std::ptr::copy_nonoverlapping(
                &elem as *const T as *const u8,
                ret.storage.as_mut_ptr(),
                mem::size_of::<T>());
            ret
        }
    }

    pub fn reference(&self) -> &T {
        unsafe { &*(self.storage.as_ptr() as *const T) }
    }
}

#[derive(Debug)]
pub enum ArtNode<K, V> {
    Empty,

    Inner4(Box<ArtNode4<K, V>>),
    Inner16(Box<ArtNode16<K, V>>),
    Inner48(Box<ArtNode48<K, V>>),
    Inner256(Box<ArtNode256<K, V>>),

    LeafLarge(Box<(K,V)>),
    LeafLargeKey(Box<K>, SmallStruct<V>),
    LeafLargeValue(SmallStruct<K>, Box<V>),
    LeafSmall(SmallStruct<K>, SmallStruct<V>),
}

#[derive(Debug)]
pub struct ArtNodeBase {
    pub num_children: u16,
    pub partial_len: usize,
    pub partial: [u8; MAX_PREFIX_LEN],
}

#[derive(Debug)]
pub struct ArtNode4<K, V> {
    pub n: ArtNodeBase,
    pub keys: mem::ManuallyDrop<[u8; 4]>,
    pub children: mem::ManuallyDrop<[ArtNode<K, V>; 4]>,
}

#[derive(Debug)]
pub struct ArtNode16<K, V> {
    pub n: ArtNodeBase,
    pub keys: mem::ManuallyDrop<[u8; 16]>,
    pub children: mem::ManuallyDrop<[ArtNode<K, V>; 16]>,
}

pub struct ArtNode48<K, V> {
    pub n: ArtNodeBase,
    pub keys: [u8; 256],
    pub children: mem::ManuallyDrop<[ArtNode<K, V>; 48]>,
}

pub struct ArtNode256<K, V> {
    pub n: ArtNodeBase,
    pub children: [ArtNode<K, V>; 256],
}

pub trait ArtNodeTrait<K, V> {
    fn add_child(&mut self, node: ArtNode<K, V>, byte: u8);

    #[inline]
    fn is_full(&self) -> bool;

    fn grow_and_add(self, leaf: ArtNode<K, V>, byte: u8) -> ArtNode<K, V>;

    #[inline]
    fn mut_base(&mut self) -> &mut ArtNodeBase;

    #[inline]
    fn base(&self) -> &ArtNodeBase;

    #[inline]
    fn find_child_mut(&mut self, byte: u8) -> &mut ArtNode<K, V>;

    #[inline]
    fn find_child(&self, byte: u8) -> Option<&ArtNode<K, V>>;

    #[inline]
    fn has_child(&self, byte: u8) -> bool;

    #[inline]
    fn to_art_node(self: Box<Self>) -> ArtNode<K, V>;
}

impl<K: ArtKey, V> ArtNode<K, V> {
    #[inline]
    pub fn key(&self) -> &K {
        match self {
            &ArtNode::LeafLarge(ref ptr) => &ptr.0,
            &ArtNode::LeafLargeKey(ref key_ptr, _) => &*key_ptr,
            &ArtNode::LeafLargeValue(ref key_small, _) => key_small.reference(),
            &ArtNode::LeafSmall(ref key_small, _) => key_small.reference(),
            _ => panic!("Does not contain key"),
        }
    }

    #[inline]
    pub fn new_leaf(key: K, value: V) -> ArtNode<K,V> {
        if mem::size_of::<K>() > SMALL_STRUCT {
            if mem::size_of::<V>() > SMALL_STRUCT {
                ArtNode::LeafLarge(Box::new((key,value)))
            } else {
                ArtNode::LeafLargeKey(Box::new(key), SmallStruct::new(value))
            }
        } else {
            if mem::size_of::<V>() > SMALL_STRUCT {
                ArtNode::LeafLargeValue(SmallStruct::new(key), Box::new(value))
            } else {
                ArtNode::LeafSmall(SmallStruct::new(key), SmallStruct::new(value))
            }
        }
    }
}

impl ArtNodeBase {
    pub fn new() -> Self {
        ArtNodeBase {
            num_children: 0,
            partial_len: 0,
            partial: unsafe { mem::uninitialized() }
        }
    }

    pub fn compute_prefix_match<K: ArtKey>(&self, key: &K, depth: usize) -> usize {
        for i in 0..self.partial_len {
            if key.bytes()[i + depth] != self.partial[i] {
                return i;
            }
        }
        self.partial_len
    }
}

impl<K, V> ArtNode4<K, V> {
    pub fn new() -> Self {
        ArtNode4 {
            n: ArtNodeBase::new(),
            keys: unsafe { mem::uninitialized() },
            children: unsafe { mem::uninitialized() },
        }
    }
}

impl<K,V> Drop for ArtNode4<K,V> {
    fn drop(&mut self) {
        for i in 0..self.n.num_children {
            drop(&mut self.children[i as usize]);
        }
    }
}

impl<K, V> ArtNode16<K, V> {
    pub fn new() -> Self {
        ArtNode16 {
            n: ArtNodeBase::new(),
            keys: unsafe { mem::uninitialized() },
            children: unsafe { mem::uninitialized() }
        }
    }
}

impl<K,V> Drop for ArtNode16<K,V> {
    fn drop(&mut self) {
        for i in 0..self.n.num_children {
            drop(&mut self.children[i as usize]);
        }
    }
}

impl<K, V> ArtNode48<K, V> {
    pub fn new() -> Self {
        ArtNode48 {
            n: ArtNodeBase::new(),
            keys: [EMPTY_CELL; 256],
            children: unsafe { mem::uninitialized() }
        }
    }
}

impl<K,V> Drop for ArtNode48<K,V> {
    fn drop(&mut self) {
        for i in 0..256 {
            if self.keys[i] != EMPTY_CELL {
                drop(&mut self.children[self.keys[i] as usize - 1]);
            }
        }
    }
}

impl<K, V> ArtNode256<K, V> {
    pub fn new() -> Self {
        ArtNode256 {
            n: ArtNodeBase::new(),
            children: unsafe { make_array!(256, ArtNode::Empty) }
        }
    }
}

impl<K: ArtKey, V> ArtNodeTrait<K, V> for ArtNode4<K, V> {
    fn add_child(&mut self, child: ArtNode<K, V>, byte: u8) {
        let idx = self.n.num_children as usize;
        unsafe {
            ptr::write(&mut self.children[idx] as *mut ArtNode<K,V>, child);
            ptr::write(&mut self.keys[idx] as *mut u8, byte);
        }
        self.n.num_children += 1;
    }

    fn is_full(&self) -> bool {
        self.n.num_children >= 4
    }

    fn to_art_node(self: Box<Self>) -> ArtNode<K,V> {
        ArtNode::Inner4(self)
    }

    fn grow_and_add(mut self, leaf: ArtNode<K, V>, byte: u8) -> ArtNode<K, V> {
        let mut new_node = Box::new(ArtNode16::new());
        new_node.n.partial_len = self.n.partial_len;

        unsafe {
            ptr::copy_nonoverlapping(
                self.n.partial.as_ptr(),
                new_node.n.partial.as_mut_ptr(),
                self.n.partial.len());
        }

        new_node.add_child(leaf, byte);

        for i in 0..4 {
            let child = std::mem::replace(&mut self.children[i as usize], ArtNode::Empty);
            new_node.add_child(child, self.keys[i as usize]);
        }

        ArtNode::Inner16(new_node)
    }

    fn mut_base(&mut self) -> &mut ArtNodeBase {
        &mut self.n
    }

    fn base(&self) -> &ArtNodeBase {
        &self.n
    }

    fn find_child_mut(&mut self, byte: u8) -> &mut ArtNode<K, V> {
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return &mut self.children[i as usize];
            }
        }
        panic!("No requested child");
    }

    fn find_child(&self, byte: u8) -> Option<&ArtNode<K, V>> {
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return Some(&self.children[i as usize]);
            }
        }
        None
    }

    fn has_child(&self, byte: u8) -> bool {
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return true;
            }
        }
        false
    }
}

impl<K: ArtKey, V> ArtNodeTrait<K, V> for ArtNode16<K, V> {
    fn add_child(&mut self, child: ArtNode<K, V>, byte: u8) {
        let idx = self.n.num_children as usize;
        unsafe {
            ptr::write(&mut self.children[idx] as *mut ArtNode<K,V>, child);
            ptr::write(&mut self.keys[idx] as *mut u8, byte);
        }
        self.n.num_children += 1;
    }

    fn is_full(&self) -> bool {
        self.n.num_children >= 16
    }

    fn to_art_node(self: Box<Self>) -> ArtNode<K,V> {
        ArtNode::Inner16(self)
    }

    fn grow_and_add(mut self, leaf: ArtNode<K, V>, byte: u8) -> ArtNode<K, V> {
        let mut new_node = Box::new(ArtNode48::new());
        new_node.n.partial_len = self.n.partial_len;

        unsafe {
            ptr::copy_nonoverlapping(
                self.n.partial.as_ptr(),
                new_node.n.partial.as_mut_ptr(),
                self.n.partial.len());
        }

        new_node.add_child(leaf, byte);

        for i in 0..16 {
            let child = std::mem::replace(&mut self.children[i], ArtNode::Empty);
            new_node.add_child(child, self.keys[i]);
        }

        ArtNode::Inner48(new_node)
    }

    fn mut_base(&mut self) -> &mut ArtNodeBase {
        &mut self.n
    }

    fn base(&self) -> &ArtNodeBase {
        &self.n
    }

    fn find_child_mut(&mut self, byte: u8) -> &mut ArtNode<K, V> {
        // TODO: O(1)
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return &mut self.children[i as usize];
            }
        }
        panic!("No requested child");
    }

    fn find_child(&self, byte: u8) -> Option<&ArtNode<K, V>> {
        // TODO: O(1)
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return Some(&self.children[i as usize]);
            }
        }
        None
    }

    fn has_child(&self, byte: u8) -> bool {
        for i in 0..self.n.num_children {
            if self.keys[i as usize] == byte {
                return true;
            }
        }
        false
    }
}

impl<K: ArtKey, V> ArtNodeTrait<K, V> for ArtNode48<K, V> {
    fn add_child(&mut self, child: ArtNode<K, V>, byte: u8) {
        unsafe {
            let idx = self.n.num_children as usize;
            ptr::write(&mut self.children[idx] as *mut ArtNode<K,V>, child);
        }
        self.n.num_children += 1;
        self.keys[byte as usize] = self.n.num_children as u8;
    }

    fn is_full(&self) -> bool {
        self.n.num_children >= 48
    }

    fn to_art_node(self: Box<Self>) -> ArtNode<K,V> {
        ArtNode::Inner48(self)
    }

    fn grow_and_add(mut self, leaf: ArtNode<K, V>, byte: u8) -> ArtNode<K, V> {
        let mut new_node = Box::new(ArtNode256::new());

        new_node.n.partial_len = self.n.partial_len;

        unsafe {
            ptr::copy_nonoverlapping(
                self.n.partial.as_ptr(),
                new_node.n.partial.as_mut_ptr(),
                self.n.partial.len());
        }

        new_node.add_child(leaf, byte);

        for i in 0..256 {
            if self.keys[i] != EMPTY_CELL {
                let child = std::mem::replace(&mut self.children[self.keys[i] as usize - 1], ArtNode::Empty);
                new_node.add_child(child, i as u8);
            }
        }

        ArtNode::Inner256(new_node)
    }

    fn mut_base(&mut self) -> &mut ArtNodeBase {
        &mut self.n
    }

    fn base(&self) -> &ArtNodeBase {
        &self.n
    }

    fn find_child_mut(&mut self, byte: u8) -> &mut ArtNode<K, V> {
        &mut self.children[self.keys[byte as usize] as usize - 1]
    }

    fn find_child(&self, byte: u8) -> Option<&ArtNode<K, V>> {
        if self.keys[byte as usize] == EMPTY_CELL {
            None
        } else {
            Some(&self.children[self.keys[byte as usize] as usize - 1])
        }
    }

    fn has_child(&self, byte: u8) -> bool {
        self.keys[byte as usize] != EMPTY_CELL
    }
}

impl<K: ArtKey, V> ArtNodeTrait<K, V> for ArtNode256<K, V> {
    fn add_child(&mut self, child: ArtNode<K, V>, byte: u8) {
        self.n.num_children += 1;
        self.children[byte as usize] = child;
    }

    fn is_full(&self) -> bool {
        self.n.num_children >= 256
    }

    fn to_art_node(self: Box<Self>) -> ArtNode<K,V> {
        ArtNode::Inner256(self)
    }

    fn grow_and_add(self, _leaf: ArtNode<K, V>, _byte: u8) -> ArtNode<K, V> {
        panic!("Cannot grow 256 sized node");
    }

    fn mut_base(&mut self) -> &mut ArtNodeBase {
        &mut self.n
    }

    fn base(&self) -> &ArtNodeBase {
        &self.n
    }

    fn find_child_mut(&mut self, byte: u8) -> &mut ArtNode<K, V> {
        &mut self.children[byte as usize]
    }

    fn find_child(&self, byte: u8) -> Option<&ArtNode<K, V>> {
        match &self.children[byte as usize] {
            &ArtNode::Empty => None,
            value => Some(value),
        }
    }

    fn has_child(&self, byte: u8) -> bool {
        match self.children[byte as usize] {
            ArtNode::Empty => false,
            _ => true,
        }
    }
}

impl<K, T> std::fmt::Debug for ArtNode48<K, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ArtNode48{{ n: {:?}, keys, children }}", self.n)
    }
}

impl<K, T> std::fmt::Debug for ArtNode256<K, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ArtNode256{{ n: {:?}, keys, children }}", self.n)
    }
}

impl<_K, _V> Default for ArtNode<_K, _V> {
    fn default() -> Self {
        ArtNode::Empty
    }
}
