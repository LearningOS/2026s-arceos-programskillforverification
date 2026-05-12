#![no_std]

use allocator::{AllocError, BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
    count: usize,
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0,
        }
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = start + size;
        self.count = 0;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        Ok(())
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        // 1. 把 b_pos 向上对齐到 layout.align()
        //    (align 一定是 2 的幂,用位运算即可)
        let aligned = (self.b_pos + align - 1) & !(align - 1);

        // 2. 计算分配后新的 b_pos,并检查是否撞到 p_pos
        let new_b_pos = aligned.checked_add(size).ok_or(AllocError::NoMemory)?;
        if new_b_pos > self.p_pos {
            return Err(AllocError::NoMemory);
        }

        // 3. 推进 b_pos,记一次活跃分配
        self.b_pos = new_b_pos;
        self.count += 1;

        // 4. 把 usize 地址包成 NonNull<u8> 返回
        //    aligned 不可能是 0(start 是有效物理/虚拟地址),所以 new_unchecked 安全
        Ok(unsafe { core::ptr::NonNull::new_unchecked(aligned as *mut u8) })
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        self.count -= 1;
        if self.count == 0 {
            self.b_pos = self.start; // 整段 bytes 区一次性回收
        }
    }

    fn total_bytes(&self) -> usize {
        self.p_pos - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(
        &mut self,
        num_pages: usize,
        align_pow2: usize,
    ) -> allocator::AllocResult<usize> {
        // 1) 参数校验
        if !align_pow2.is_power_of_two() || align_pow2 < Self::PAGE_SIZE {
            return Err(AllocError::InvalidParam);
        }

        // 2) 需要的字节数
        let size = num_pages
            .checked_mul(Self::PAGE_SIZE)
            .ok_or(AllocError::NoMemory)?;

        // 3) 从 p_pos 往下减 size,再把起点向下对齐
        let new_p_pos =
            self.p_pos.checked_sub(size).ok_or(AllocError::NoMemory)? & !(align_pow2 - 1);

        // 4) 不能撞上 bytes 区
        if new_p_pos < self.b_pos {
            return Err(AllocError::NoMemory);
        }

        // 5) 推进 p_pos,返回新分配区间的起始地址
        self.p_pos = new_p_pos;
        Ok(new_p_pos)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        unimplemented!("EarlyAllocator 不支持回收页")
    }

    fn total_pages(&self) -> usize {
        (self.end - self.b_pos) / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / Self::PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / Self::PAGE_SIZE
    }
}
