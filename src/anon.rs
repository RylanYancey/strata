
use std::ptr;
use std::ptr::NonNull;
use std::alloc::Layout;
use std::alloc::{alloc, realloc};
use std::mem::needs_drop;

use strata_traits::Component;

use crate::archetypes::ComponentId;

/// Anonymously-Typed Vector
pub struct AnonVec {
    inner: NonNull<u8>,
    layout: Layout,
    capacity: usize,
    len: usize,
    drop: Option<fn(*mut u8)>,
    cmpid: ComponentId,
}

impl AnonVec {
    pub fn new(anon: Anon) -> Self {
        Self {
            inner: anon.inner,
            layout: anon.layout,
            capacity: 1,
            len: 1,
            drop: anon.drop,
            cmpid: anon.cmpid,
        }
    }

    /// Append a value to the back of the vector. 
    pub fn push(&mut self, val: Anon) {
        unsafe {
            if val.cmpid != self.cmpid {
                panic!("cmpids did not match!")
            }

            // Allocate space as needed.
            self.grow_if_full();

            let size = self.layout.size();

            // Copy `size` bytes from `val.ptr` to `inner.ptr + size * len`.
            ptr::copy(val.as_ptr(), self.inner.as_ptr().add(size * self.len), size);

            // increment the length
            self.len += 1;
        }
    }

    /// Returns a pointer to the value at Index.
    pub fn index(&self, index: usize) -> Anon {
        unsafe {
            // panic if out of bounds
            if index >= self.len {
                panic!("Index ({0}) must be less than the len! (len: ({1})", index, self.len);
            }

            let size = self.layout.size();

            // the location of the value
            let src = self.inner.as_ptr().add(size * index);

            Anon {
                inner: NonNull::new(src).unwrap(),
                drop: self.drop,
                cmpid: self.cmpid,
                layout: self.layout
            }
        }
    }

    pub fn index_cast<T>(&mut self, index: usize) -> &'static mut T 
    where
        T: Component
    {
        unsafe {
            if index >= self.len {
                panic!("Index ({0}) must be less than the len! (len: ({1})", index, self.len);
            }

            let size = self.layout.size();

            &mut *(self.inner.as_ptr().add(size * index).cast::<T>())
        }
    }

    /// Swaps the last element with the element at Index, then destroys the value.
    pub fn destroy_swap(&mut self, index: usize) {
        unsafe {
            // panic on out of bounds
            if index >= self.len {
                panic!("Index ({0}) must be less than the len! (len: {1})", index, self.len);
            }

            let size = self.layout.size();

            // if this is the last element, drop and decrement.
            if index == self.len - 1 {
                // Drop the value inside if it needs it using the
                // function we created for it earlier in Anon.
                if let Some(drop) = self.drop {
                    drop(self.inner.as_ptr().add(size * self.len));
                }
                // decrement to overwrite the value
                self.len -= 1;
                return;
            }

            // location to copy from (last element)
            let src = self.inner.as_ptr().add(size * self.len);
            // location at index
            let dst = self.inner.as_ptr().add(size * index);

            // Drop the value inside if it needs it using the
            // function we created for it earlier. 
            if let Some(drop) = self.drop {
                drop(self.inner.as_ptr().add(size * self.len));
            }

            // perform the copy to overwrite the memory
            ptr::copy(src, dst, size);

            self.len -= 1;
        }
    }

    pub fn destroy_nodrop(&mut self, index: usize) {
        unsafe {
            // panic on out of bounds
            if index >= self.len {
                panic!("Index ({0}) must be less than the len! (len: {1})", index, self.len);
            }

            let size = self.layout.size();

            // if this is the last element, drop and decrement.
            if index == self.len - 1 {
                // decrement to overwrite the value
                self.len -= 1;
                return;
            }

            // location to copy from (last element)
            let src = self.inner.as_ptr().add(size * self.len);
            // location at index
            let dst = self.inner.as_ptr().add(size * index);

            // perform the copy to overwrite the memory
            ptr::copy(src, dst, size);

            self.len -= 1;
        }
    }

    pub fn iter_as<T>(&self) -> AnonIter<T> 
    where
        T: Component
    {
        AnonIter {
            ptr: self.inner.as_ptr().cast::<T>(),
            curr: 0,
            len: self.len,
        }
    }

    unsafe fn grow_if_full(&mut self) {
        // calculate how much space is available
        let available_space = self.capacity - self.len;

        // If there is no available space, double it.
        // Also, don't allocate anything if this is a ZST.
        if available_space == 0 && self.layout.size() != 0 {
            // Double the current capacity
            let new_capacity = self.capacity * 2;
            // Create a layout by duplicating an item for n times
            let new_layout = Layout::from_size_align(
                self.layout.size() * new_capacity,
                self.layout.align(),
            ).unwrap();

            // Reassign self.data. 
            let new_data;
                if self.capacity == 0 {
                    // if uninit, init
                    new_data = alloc(new_layout);
                } else {
                    new_data = realloc(self.inner.as_ptr(), new_layout, new_capacity * self.layout.size());
                };
            
            self.inner = NonNull::new(new_data).unwrap();
            self.capacity = new_capacity;
        }
    }

    pub fn clear(&mut self) {
        let len = self.len;
        let size = self.layout.size();

        self.len = 0;

        if let Some(drop) = self.drop {
            for i in 0..len {
                unsafe { drop(self.inner.as_ptr().add(i * size)) }
            }
        }
    }
}

pub struct Anon {
    inner: NonNull<u8>,
    drop: Option<fn(*mut u8)>,
    cmpid: ComponentId,
    layout: Layout,
}

impl Anon {
    pub fn new<T>(val: T) -> Self 
    where
        T: Component,
    {
        fn drop_as<T>(ptr: *mut u8) {
            unsafe {
                ptr.cast::<T>().drop_in_place();
            }
        }

        unsafe {
            let layout = Layout::new::<T>();
            let ptr = NonNull::new(alloc(layout)).unwrap();
            let drop: Option<fn(*mut u8)> = 
                if needs_drop::<T>() { Some(drop_as::<T>) } else { None };

            ptr::write(ptr.as_ptr().cast::<T>(), val);

            Self {
                inner: ptr,
                drop,
                cmpid: T::__internal_id(),
                layout,
            }
        }
    }

    pub fn uninit() -> Self {
        Self {
            inner: NonNull::dangling(),
            drop: None,
            cmpid: 0,
            layout: Layout::new::<i32>(),
        }
    }

    pub fn id(&self) -> ComponentId {
        self.cmpid
    }

    pub fn downcast<T>(&self) -> &T 
    where
        T: Component,
    {
        unsafe { &*self.inner.as_ptr().cast::<T>() }
    }

    pub fn downcast_mut<T>(&self) -> &mut T 
    where
        T: Component,
    {
        unsafe { &mut *self.inner.as_ptr().cast::<T>() }
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.inner.as_ptr()
    }

    pub fn clear(&self) {
        if let Some(drop) = self.drop {
            drop(self.as_ptr())
        }
    }
}

pub struct AnonIter<T: 'static> {
    pub(crate) ptr: *mut T,
    pub(crate) curr: usize,
    pub(crate) len: usize,
}

impl<T> Iterator for AnonIter<T> {
    type Item = &'static mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == self.len {
            None
        } else {
            self.curr += 1;
            unsafe { Some(&mut *self.ptr.add(self.curr - 1)) }
        }
    }
}

pub struct AnonIterChain<T: 'static> {
    pub iters: Vec<AnonIter<T>>,
}

impl<T> AnonIterChain<T> {
    pub fn is_empty(&self) -> bool {
        self.iters.is_empty()
    }

    pub fn new() -> Self {
        Self {
            iters: Vec::with_capacity(4),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            iters: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, iter: AnonIter<T>) {
        self.iters.push(iter);
    }
}

impl<T> Iterator for AnonIterChain<T> {
    type Item = &'static mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // get the last iter if it exists
        if let Some(curr) = self.iters.last_mut() {
            let out = curr.next();

            // if iters is empty now, move to the next one. 
            if curr.curr == curr.len {
                self.iters.pop();
            }

            out   
        } else {
            // else, this iter is done. 
            None
        }
    }
}

unsafe impl Send for Anon {}
unsafe impl Sync for Anon {}