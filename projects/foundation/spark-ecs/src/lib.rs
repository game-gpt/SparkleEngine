//! Spark ECS：Archetype 连续存储、查询、Resource、系统调度。
//!
//! **Component** 为纯数据；系统为纯逻辑。界面树在 `spark-widget`，不进本 crate。

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// 任意 `'static + Send + Sync` 类型均可作 Component。
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// 稳定实体 ID：低 32 位槽位，高 32 位世代。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

impl Entity {
    fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | u64::from(index))
    }

    pub fn index(self) -> u32 {
        self.0 as u32
    }

    pub fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub fn to_bits(self) -> u64 {
        self.0
    }

    pub fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

#[derive(Clone, Copy)]
struct EntityLoc {
    archetype: u32,
    row: u32,
}

struct Column {
    data: Box<dyn Any + Send + Sync>,
    swap_remove: fn(&mut dyn Any, usize),
    /// 从 src[row] 取出并追加到 dst 末尾（两列同为 `Vec<T>`）。
    take_push: fn(&mut dyn Any, usize, &mut dyn Any),
}

impl Column {
    fn new<T: Component>() -> Self {
        Self {
            data: Box::new(Vec::<T>::new()),
            swap_remove: |any, row| {
                any.downcast_mut::<Vec<T>>()
                    .expect("列类型")
                    .swap_remove(row);
            },
            take_push: |src, row, dst| {
                // 不变式：src/dst 为不同 `Vec<T>`，无别名。
                let s = src.downcast_mut::<Vec<T>>().expect("源列");
                let d = dst.downcast_mut::<Vec<T>>().expect("目标列");
                let v = s.swap_remove(row);
                d.push(v);
            },
        }
    }

    fn vec<T: Component>(&self) -> &Vec<T> {
        self.data.downcast_ref::<Vec<T>>().expect("列类型")
    }

    fn vec_mut<T: Component>(&mut self) -> &mut Vec<T> {
        self.data.downcast_mut::<Vec<T>>().expect("列类型")
    }
}

struct Archetype {
    type_ids: Vec<TypeId>,
    entities: Vec<Entity>,
    columns: Vec<Column>,
    col_of: HashMap<TypeId, usize>,
}

impl Archetype {
    fn empty() -> Self {
        Self {
            type_ids: Vec::new(),
            entities: Vec::new(),
            columns: Vec::new(),
            col_of: HashMap::new(),
        }
    }

    fn with_types(ids: Vec<TypeId>, ctors: Vec<fn() -> Column>) -> Self {
        let mut col_of = HashMap::with_capacity(ids.len());
        let mut columns = Vec::with_capacity(ctors.len());
        for (i, (id, ctor)) in ids.iter().copied().zip(ctors).enumerate() {
            col_of.insert(id, i);
            columns.push(ctor());
        }
        Self {
            type_ids: ids,
            entities: Vec::new(),
            columns,
            col_of,
        }
    }

    fn len(&self) -> usize {
        self.entities.len()
    }
}

/// 资源表（每类型至多一份）。
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast::<T>().ok().map(|b| *b))
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }
}

/// ECS 世界。
pub struct World {
    archetypes: Vec<Archetype>,
    locations: Vec<Option<EntityLoc>>,
    generations: Vec<u32>,
    free: Vec<u32>,
    archetype_index: HashMap<Vec<TypeId>, usize>,
    ctors: HashMap<TypeId, fn() -> Column>,
    pub resources: Resources,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            archetypes: vec![Archetype::empty()],
            locations: Vec::new(),
            generations: Vec::new(),
            free: Vec::new(),
            archetype_index: HashMap::new(),
            ctors: HashMap::new(),
            resources: Resources::default(),
        };
        world.archetype_index.insert(Vec::new(), 0);
        world
    }

    fn register_ctor<T: Component>(&mut self) {
        self.ctors
            .entry(TypeId::of::<T>())
            .or_insert(Column::new::<T>);
    }

    fn ensure_archetype(&mut self, types: Vec<TypeId>) -> usize {
        if let Some(&idx) = self.archetype_index.get(&types) {
            return idx;
        }
        let ctors: Vec<fn() -> Column> = types
            .iter()
            .map(|t| *self.ctors.get(t).expect("Component 构造器未注册"))
            .collect();
        let idx = self.archetypes.len();
        self.archetypes
            .push(Archetype::with_types(types.clone(), ctors));
        self.archetype_index.insert(types, idx);
        idx
    }

    fn alloc_entity(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let generation = self.generations[index as usize];
            Entity::new(index, generation)
        } else {
            let index = self.locations.len() as u32;
            self.locations.push(None);
            self.generations.push(1);
            Entity::new(index, 1)
        }
    }

    fn set_loc(&mut self, entity: Entity, loc: EntityLoc) {
        self.locations[entity.index() as usize] = Some(loc);
    }

    fn invalidate(&mut self, entity: Entity) {
        let i = entity.index() as usize;
        self.locations[i] = None;
        self.generations[i] = self.generations[i].wrapping_add(1);
        self.free.push(entity.index());
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let i = entity.index() as usize;
        matches!(self.locations.get(i), Some(Some(_)))
            && self.generations.get(i).copied() == Some(entity.generation())
    }

    pub fn entity_count(&self) -> usize {
        self.locations.iter().filter(|l| l.is_some()).count()
    }

    pub fn entity_count_hint(&self) -> u64 {
        self.entity_count() as u64
    }

    pub fn spawn_empty(&mut self) -> Entity {
        let entity = self.alloc_entity();
        let row = self.archetypes[0].len() as u32;
        self.archetypes[0].entities.push(entity);
        self.set_loc(
            entity,
            EntityLoc {
                archetype: 0,
                row,
            },
        );
        entity
    }

    pub fn spawn<T: Component>(&mut self, component: T) -> Entity {
        self.register_ctor::<T>();
        let entity = self.alloc_entity();
        let types = vec![TypeId::of::<T>()];
        let arch_idx = self.ensure_archetype(types);
        let row = self.archetypes[arch_idx].len() as u32;
        {
            let arch = &mut self.archetypes[arch_idx];
            arch.entities.push(entity);
            arch.columns[0].vec_mut::<T>().push(component);
        }
        self.set_loc(
            entity,
            EntityLoc {
                archetype: arch_idx as u32,
                row,
            },
        );
        entity
    }

    pub fn spawn2<A: Component, B: Component>(&mut self, a: A, b: B) -> Entity {
        self.register_ctor::<A>();
        self.register_ctor::<B>();
        let entity = self.alloc_entity();
        let mut types = vec![TypeId::of::<A>(), TypeId::of::<B>()];
        types.sort_unstable();
        let arch_idx = self.ensure_archetype(types);
        let row = self.archetypes[arch_idx].len() as u32;
        {
            let arch = &mut self.archetypes[arch_idx];
            arch.entities.push(entity);
            let ia = arch.col_of[&TypeId::of::<A>()];
            let ib = arch.col_of[&TypeId::of::<B>()];
            arch.columns[ia].vec_mut::<A>().push(a);
            arch.columns[ib].vec_mut::<B>().push(b);
        }
        self.set_loc(
            entity,
            EntityLoc {
                archetype: arch_idx as u32,
                row,
            },
        );
        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let loc = self.locations[entity.index() as usize].unwrap();
        let swapped = self.swap_remove_row(loc.archetype as usize, loc.row as usize);
        if let Some(moved) = swapped {
            self.set_loc(
                moved,
                EntityLoc {
                    archetype: loc.archetype,
                    row: loc.row,
                },
            );
        }
        self.invalidate(entity);
        true
    }

    /// 删除行；若发生交换则返回被换入该行的实体。
    fn swap_remove_row(&mut self, arch_idx: usize, row: usize) -> Option<Entity> {
        let arch = &mut self.archetypes[arch_idx];
        let last = arch.entities.len() - 1;
        arch.entities.swap_remove(row);
        for col in &mut arch.columns {
            (col.swap_remove)(col.data.as_mut(), row);
        }
        if row < last {
            Some(arch.entities[row])
        } else {
            None
        }
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        let loc = self.locations[entity.index() as usize]?;
        let arch = &self.archetypes[loc.archetype as usize];
        let col = *arch.col_of.get(&TypeId::of::<T>())?;
        arch.columns[col].vec::<T>().get(loc.row as usize)
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        let loc = self.locations[entity.index() as usize].unwrap();
        let arch = &mut self.archetypes[loc.archetype as usize];
        let col = *arch.col_of.get(&TypeId::of::<T>())?;
        arch.columns[col].vec_mut::<T>().get_mut(loc.row as usize)
    }

    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        if self.get::<T>(entity).is_some() {
            *self.get_mut::<T>(entity).unwrap() = value;
            return true;
        }
        self.register_ctor::<T>();
        let loc = self.locations[entity.index() as usize].unwrap();
        let src_idx = loc.archetype as usize;
        let row = loc.row as usize;
        let mut new_types = self.archetypes[src_idx].type_ids.clone();
        new_types.push(TypeId::of::<T>());
        new_types.sort_unstable();
        let dst_idx = self.ensure_archetype(new_types);
        self.migrate(entity, src_idx, row, dst_idx, Some(value));
        true
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        let loc = self.locations[entity.index() as usize].unwrap();
        let src_idx = loc.archetype as usize;
        let row = loc.row as usize;
        if !self.archetypes[src_idx]
            .col_of
            .contains_key(&TypeId::of::<T>())
        {
            return None;
        }
        let mut new_types = self.archetypes[src_idx].type_ids.clone();
        new_types.retain(|t| *t != TypeId::of::<T>());
        let dst_idx = self.ensure_archetype(new_types);
        self.migrate_except::<T>(entity, src_idx, row, dst_idx)
    }

    fn migrate<T: Component>(
        &mut self,
        entity: Entity,
        src_idx: usize,
        row: usize,
        dst_idx: usize,
        new_value: Option<T>,
    ) {
        let src_types = self.archetypes[src_idx].type_ids.clone();
        let dst_row = self.archetypes[dst_idx].len() as u32;

        for tid in &src_types {
            let src_col = self.archetypes[src_idx].col_of[tid];
            let dst_col = self.archetypes[dst_idx].col_of[tid];
            // 分属不同 archetype，列缓冲无别名。
            let (src_arch, dst_arch) = two_mut(&mut self.archetypes, src_idx, dst_idx);
            let take = src_arch.columns[src_col].take_push;
            take(
                src_arch.columns[src_col].data.as_mut(),
                row,
                dst_arch.columns[dst_col].data.as_mut(),
            );
        }

        if let Some(v) = new_value {
            let dst = &mut self.archetypes[dst_idx];
            let ic = dst.col_of[&TypeId::of::<T>()];
            dst.columns[ic].vec_mut::<T>().push(v);
        }

        self.archetypes[dst_idx].entities.push(entity);

        let swapped = {
            let src = &mut self.archetypes[src_idx];
            let last = src.entities.len() - 1;
            src.entities.swap_remove(row);
            // 各列已在 take_push 中 swap_remove(row)
            if row < last {
                Some(src.entities[row])
            } else {
                None
            }
        };
        if let Some(moved) = swapped {
            self.set_loc(
                moved,
                EntityLoc {
                    archetype: src_idx as u32,
                    row: row as u32,
                },
            );
        }
        self.set_loc(
            entity,
            EntityLoc {
                archetype: dst_idx as u32,
                row: dst_row,
            },
        );
    }

    fn migrate_except<T: Component>(
        &mut self,
        entity: Entity,
        src_idx: usize,
        row: usize,
        dst_idx: usize,
    ) -> Option<T> {
        let skip = TypeId::of::<T>();
        let src_types = self.archetypes[src_idx].type_ids.clone();
        let dst_row = self.archetypes[dst_idx].len() as u32;
        let mut taken = None;

        for tid in &src_types {
            let src_col = self.archetypes[src_idx].col_of[tid];
            if *tid == skip {
                let src = &mut self.archetypes[src_idx];
                taken = Some(src.columns[src_col].vec_mut::<T>().swap_remove(row));
                continue;
            }
            let dst_col = self.archetypes[dst_idx].col_of[tid];
            let (src_arch, dst_arch) = two_mut(&mut self.archetypes, src_idx, dst_idx);
            let take = src_arch.columns[src_col].take_push;
            take(
                src_arch.columns[src_col].data.as_mut(),
                row,
                dst_arch.columns[dst_col].data.as_mut(),
            );
        }

        self.archetypes[dst_idx].entities.push(entity);
        let swapped = {
            let src = &mut self.archetypes[src_idx];
            let last = src.entities.len() - 1;
            src.entities.swap_remove(row);
            if row < last {
                Some(src.entities[row])
            } else {
                None
            }
        };
        if let Some(moved) = swapped {
            self.set_loc(
                moved,
                EntityLoc {
                    archetype: src_idx as u32,
                    row: row as u32,
                },
            );
        }
        self.set_loc(
            entity,
            EntityLoc {
                archetype: dst_idx as u32,
                row: dst_row,
            },
        );
        taken
    }

    pub fn for_each<T: Component>(&self, mut f: impl FnMut(Entity, &T)) {
        let tid = TypeId::of::<T>();
        for arch in &self.archetypes {
            let Some(&col) = arch.col_of.get(&tid) else {
                continue;
            };
            let vec = arch.columns[col].vec::<T>();
            for (i, &e) in arch.entities.iter().enumerate() {
                f(e, &vec[i]);
            }
        }
    }

    pub fn for_each_mut<T: Component>(&mut self, mut f: impl FnMut(Entity, &mut T)) {
        let tid = TypeId::of::<T>();
        for arch in &mut self.archetypes {
            let Some(&col) = arch.col_of.get(&tid) else {
                continue;
            };
            let n = arch.entities.len();
            let entities = arch.entities.clone();
            let vec = arch.columns[col].vec_mut::<T>();
            for i in 0..n {
                f(entities[i], &mut vec[i]);
            }
        }
    }

    pub fn for_each2_mut<A: Component, B: Component>(
        &mut self,
        mut f: impl FnMut(Entity, &mut A, &mut B),
    ) {
        let ta = TypeId::of::<A>();
        let tb = TypeId::of::<B>();
        for arch in &mut self.archetypes {
            if !arch.col_of.contains_key(&ta) || !arch.col_of.contains_key(&tb) {
                continue;
            }
            let ia = arch.col_of[&ta];
            let ib = arch.col_of[&tb];
            let n = arch.entities.len();
            let entities = arch.entities.clone();
            if ia == ib {
                continue;
            }
            let (a_slice, b_slice) = if ia < ib {
                let (left, right) = arch.columns.split_at_mut(ib);
                (
                    left[ia].vec_mut::<A>().as_mut_slice(),
                    right[0].vec_mut::<B>().as_mut_slice(),
                )
            } else {
                let (left, right) = arch.columns.split_at_mut(ia);
                (
                    right[0].vec_mut::<A>().as_mut_slice(),
                    left[ib].vec_mut::<B>().as_mut_slice(),
                )
            };
            for i in 0..n {
                f(entities[i], &mut a_slice[i], &mut b_slice[i]);
            }
        }
    }
}

fn two_mut<T>(slice: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j);
    if i < j {
        let (left, right) = slice.split_at_mut(j);
        (&mut left[i], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(i);
        (&mut right[0], &mut left[j])
    }
}

/// 系统：读写集供未来并行调度；当前顺序执行。
pub trait System: Send {
    fn name(&self) -> &str {
        "system"
    }
    fn run(&mut self, world: &mut World);
    fn reads(&self) -> &[TypeId] {
        &[]
    }
    fn writes(&self) -> &[TypeId] {
        &[]
    }
}

/// 函数式系统包装。
pub struct FunctionSystem<F> {
    name: &'static str,
    f: F,
}

impl<F> FunctionSystem<F>
where
    F: FnMut(&mut World) + Send,
{
    pub fn new(name: &'static str, f: F) -> Self {
        Self { name, f }
    }
}

impl<F> System for FunctionSystem<F>
where
    F: FnMut(&mut World) + Send,
{
    fn name(&self) -> &str {
        self.name
    }

    fn run(&mut self, world: &mut World) {
        (self.f)(world);
    }
}

/// 顺序调度器（并行接口预留：按 reads/writes 分桶）。
#[derive(Default)]
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system<S: System + 'static>(&mut self, system: S) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn add_fn(
        &mut self,
        name: &'static str,
        f: impl FnMut(&mut World) + Send + 'static,
    ) -> &mut Self {
        self.add_system(FunctionSystem::new(name, f))
    }

    pub fn run(&mut self, world: &mut World) {
        for sys in &mut self.systems {
            sys.run(world);
        }
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Pos(f32);
    #[derive(Debug, PartialEq)]
    struct Vel(f32);

    #[test]
    fn spawn_get_despawn() {
        let mut w = World::new();
        let e = w.spawn(Pos(1.0));
        assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
        assert!(w.despawn(e));
        assert!(!w.is_alive(e));
        assert!(w.get::<Pos>(e).is_none());
    }

    #[test]
    fn spawn2_and_foreach() {
        let mut w = World::new();
        let e = w.spawn2(Pos(1.0), Vel(2.0));
        w.for_each2_mut::<Pos, Vel>(|_, p, v| {
            p.0 += v.0;
        });
        assert_eq!(w.get::<Pos>(e).unwrap().0, 3.0);
    }

    #[test]
    fn insert_migrates_archetype() {
        let mut w = World::new();
        let e = w.spawn(Pos(1.0));
        assert!(w.insert(e, Vel(5.0)));
        assert_eq!(w.get::<Vel>(e).unwrap().0, 5.0);
        assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
    }

    #[test]
    fn remove_component() {
        let mut w = World::new();
        let e = w.spawn2(Pos(1.0), Vel(2.0));
        let v = w.remove::<Vel>(e).unwrap();
        assert_eq!(v.0, 2.0);
        assert!(w.get::<Vel>(e).is_none());
        assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
    }

    #[test]
    fn resources_and_schedule() {
        let mut w = World::new();
        w.resources.insert(7u32);
        let e = w.spawn(Pos(0.0));
        let mut sched = Schedule::new();
        sched.add_fn("bump", |world| {
            let add = *world.resources.get::<u32>().unwrap();
            world.for_each_mut::<Pos>(|_, p| p.0 += add as f32);
        });
        sched.run(&mut w);
        assert_eq!(w.get::<Pos>(e).unwrap().0, 7.0);
    }

    #[test]
    fn generation_rejects_stale() {
        let mut w = World::new();
        let e = w.spawn_empty();
        w.despawn(e);
        let e2 = w.spawn_empty();
        assert_ne!(e, e2);
        assert!(!w.is_alive(e));
        assert!(w.is_alive(e2));
    }
}
