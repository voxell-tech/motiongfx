use bevy::prelude::*;
use motiongfx_core_macros::tuple_combinations;

pub trait GetId {
    fn id(&self) -> Entity;
}

impl<T> GetId for (Entity, T) {
    fn id(&self) -> Entity {
        self.0
    }
}

pub trait GetMutValue<T, const N: usize> {
    fn get_mut_value(&mut self) -> &mut T;
}

impl<T> GetMutValue<T, 0> for T {
    fn get_mut_value(&mut self) -> &mut T {
        self
    }
}

pub trait GetMut<U, const N: usize> {
    fn get_mut<T>(&mut self) -> &mut T
    where
        U: GetMutValue<T, N>;
}

impl<U, const N: usize> GetMut<U, N> for (Entity, U) {
    fn get_mut<T>(&mut self) -> &mut T
    where
        U: GetMutValue<T, N>,
    {
        self.1.get_mut_value()
    }
}

macro_rules! impl_get_mut_value {
    ([$($generic:ident),+], $main_generic:ident, $number:tt) => {
        impl <$($generic),+> GetMutValue<$main_generic, $number> for ($($generic),+) {
            fn get_mut_value(&mut self) -> &mut $main_generic {
                &mut self.$number
            }
        }
    };
}

tuple_combinations!(impl_get_mut_value, 20);
