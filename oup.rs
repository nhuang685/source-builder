//! author: n685
use crate::algo_lib::input::UnsafeScanner;
use crate::algo_lib::numeric::modular::{Mint1e9_7, Modular};
use std::io;
use std::io::{BufRead, BufWriter, Write};
create_value!(V1, pub v1 : u32 = 1_000_000_006);
type Mint = Modular<V1>;
fn solve<R: BufRead, W: Write>(scan: &mut UnsafeScanner<R>, out: &mut W) {
    let a = Mint::from(5);
    let b = Mint::from(7);
    let c = Mint::from(-6);
    writeln!(out, "{}", a + b).ok();
    writeln!(out, "{}", a + c).ok();
    writeln!(out, "{}", b - c).ok();
}
fn main() {
    let mut scan = UnsafeScanner::new(io::stdin().lock());
    let mut out = BufWriter::new(io::stdout().lock());
    solve(&mut scan, &mut out);
}
pub mod algo_lib {
    pub mod input {
        //! https://github.com/EbTech/rust-algorithms/blob/master/src/scanner.rs
        use std::io;
        use std::str;
        /// Same API as Scanner but nearly twice as fast, using horribly unsafe dark arts
        pub struct UnsafeScanner<R> {
            reader: R,
            buf_str: Vec<u8>,
            buf_iter: str::SplitAsciiWhitespace<'static>,
        }
        impl<R: io::BufRead> UnsafeScanner<R> {
            pub fn new(reader: R) -> Self {
                Self {
                    reader,
                    buf_str: vec![],
                    buf_iter: "".split_ascii_whitespace(),
                }
            }
            /// This function should be marked unsafe, but noone has time for that in a
            /// programming contest. Use at your own risk!
            pub fn token<T: str::FromStr>(&mut self) -> T {
                loop {
                    if let Some(token) = self.buf_iter.next() {
                        return token.parse().ok().expect("Failed parse");
                    }
                    self.buf_str.clear();
                    self.reader
                        .read_until(b'\n', &mut self.buf_str)
                        .expect("Failed read");
                    self.buf_iter = unsafe {
                        let slice = str::from_utf8_unchecked(&self.buf_str);
                        std::mem::transmute(slice.split_ascii_whitespace())
                    };
                }
            }
        }
        pub fn scanner_from_file(
            filename: &str,
        ) -> UnsafeScanner<io::BufReader<std::fs::File>> {
            let file = std::fs::File::open(filename).expect("Input file not found");
            UnsafeScanner::new(io::BufReader::new(file))
        }
        pub fn writer_to_file(filename: &str) -> io::BufWriter<std::fs::File> {
            let file = std::fs::File::create(filename).expect("Output file not found");
            io::BufWriter::new(file)
        }
    }
    pub mod numeric {
        pub mod ext_eucl {
            use crate::algo_lib::numeric::num_traits::integer::Integer;
            pub fn ext_eucl<T: Integer>(a: T, b: T) -> Option<(T, T)> {
                if a < b {
                    ext_eucl(b, a).map(|(a, b)| (b, a))
                } else if b == T::zero() {
                    if a == T::one() { Some((a, T::zero())) } else { None }
                } else {
                    ext_eucl(b, a % b).map(|(x, y)| (y, x - (a / b) * y))
                }
            }
        }
        pub mod modular {
            use crate::create_value;
            use crate::algo_lib::numeric::ext_eucl::ext_eucl;
            use crate::algo_lib::numeric::num_traits::arithmetic::Arithmetic;
            use crate::algo_lib::numeric::num_traits::integer::Integer;
            use crate::algo_lib::numeric::num_traits::unsigned::Unsigned;
            use crate::algo_lib::util::value::Value;
            use std::fmt::{Debug, Display};
            use std::hash::Hash;
            use std::iter::{Product, Sum};
            use std::marker::PhantomData;
            use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
            pub trait BaseMod<S: Unsigned>: Arithmetic + Eq + Ord + Hash + Default {
                fn val(self) -> S;
                fn modu() -> S;
            }
            macro_rules! from_unsigned_lower {
                ($name:ident, $s:ty, $($t:ty)+) => {
                    $(impl < V : Value <$s >> From <$t > for $name < V > { fn from(value
                    : $t) -> Self { Self { v : value as $s % V::val(), phantom :
                    PhantomData } } })+
                };
            }
            macro_rules! from_unsigned_upper {
                ($name:ident, $s:ty, $($t:ty)+) => {
                    $(impl < V : Value <$s >> From <$t > for $name < V > { fn from(value
                    : $t) -> Self { Self { v : (value % V::val() as $t) as $s, phantom :
                    PhantomData } } })+
                };
            }
            macro_rules! from_signed_lower {
                ($name:ident, $s:ty, $($t:ty)+) => {
                    $(impl < V : Value <$s >> From <$t > for $name < V > { fn from(value
                    : $t) -> Self { let mut val = value as i64 % (V::val() as i64); if
                    val < 0 { val += V::val() as i64; } Self { v : val as $s, phantom :
                    PhantomData } } })+
                };
            }
            macro_rules! from_signed_upper {
                ($name:ident, $s:ty, $($t:ty)+) => {
                    $(impl < V : Value <$s >> From <$t > for $name < V > { fn from(value
                    : $t) -> Self { let mut val = value % (V::val() as $t); if val < 0 {
                    val += V::val() as $t; } Self { v : val as $s, phantom : PhantomData
                    } } })+
                };
            }
            macro_rules! mod_impl {
                ($name:ident, $s:ty) => {
                    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
                    pub struct $name < V : Value <$s >> { v : $s, phantom : PhantomData <
                    V >, } impl < V : Value <$s >> Display for $name < V > { fn fmt(&
                    self, f : & mut std::fmt::Formatter <'_ >) -> std::fmt::Result {
                    Display::fmt(& self.v, f) } } impl < V : Value <$s >> Debug for $name
                    < V > { fn fmt(& self, f : & mut std::fmt::Formatter <'_ >) ->
                    std::fmt::Result { Debug::fmt(& self.v, f) } }
                    from_unsigned_lower!($name, $s, u8 u16 u32);
                    from_unsigned_upper!($name, $s, usize u64 u128);
                    from_signed_lower!($name, $s, i8 i16 i32); from_signed_upper!($name,
                    $s, isize i64 i128); impl < V : Value <$s >> AddAssign for $name < V
                    > { fn add_assign(& mut self, rhs : Self) { self.v += rhs.v; if self
                    .v >= Self::modu() { self.v -= Self::modu(); } } } impl < V : Value
                    <$s >> Add for $name < V > { type Output = Self; fn add(mut self, rhs
                    : Self) -> Self::Output { self += rhs; self } } impl < V : Value <$s
                    >> SubAssign for $name < V > { fn sub_assign(& mut self, rhs : Self)
                    { self.v = self.v.wrapping_sub(rhs.v); if self.v >= Self::modu() {
                    self.v = self.v.wrapping_add(Self::modu()) } } } impl < V : Value <$s
                    >> Sub for $name < V > { type Output = Self; fn sub(mut self, rhs :
                    Self) -> Self::Output { self -= rhs; self } } impl < V : Value <$s >>
                    MulAssign for $name < V > { fn mul_assign(& mut self, rhs : Self) {
                    self.v = self.v.mod_mul(rhs.v, Self::modu()); } } impl < V : Value
                    <$s >> Mul for $name < V > { type Output = Self; fn mul(mut self, rhs
                    : Self) -> Self::Output { self *= rhs; self } } impl < V : Value <$s
                    >> Neg for $name < V > { type Output = Self; fn neg(self) ->
                    Self::Output { Self::from(Self::modu() - self.v) } } impl < V : Value
                    <$s >> Arithmetic for $name < V > { fn zero() -> Self { Self::raw(0)
                    } fn one() -> Self { Self::raw(1) } fn two() -> Self { Self::one() +
                    Self::one() } fn from_u8(val : u8) -> Self { Self::from(val) } fn
                    from_usize(val : usize) -> Self { Self::from(val) } } impl < V :
                    Value <$s >> $name < V > { pub fn raw(v : $s) -> Self { Self { v,
                    phantom : PhantomData, } } pub fn pow < T : Integer > (mut self, mut
                    exp : T) -> Self { let mut res = Self::one(); while exp > T::zero() {
                    if exp % T::two() == T::one() { res *= self; } self *= self; exp /=
                    T::two(); } res } pub fn inv(& self) -> Self { match self
                    .inv_checked() { Some(val) => val, None => panic!("gcd({}, {}) != 1",
                    self.v, Self::modu()), } } pub fn inv_checked(& self) -> Option <
                    Self > { ext_eucl(self.v, Self::modu()).map(| val | Self::from(val
                    .0)) } pub fn last_k(n : $s, k : $s) -> Self { (n..n - k)
                    .map(Self::from).fold(Self::one(), | a, b | a * b) } } impl < V :
                    Value <$s >> Sum for $name < V > { fn sum < I : Iterator < Item =
                    Self >> (iter : I) -> Self { iter.fold(Self::zero(), Self::add) } }
                    impl <'a, V : Value <$s >> Sum <&'a $name < V >> for $name < V > { fn
                    sum < I : Iterator < Item = &'a Self >> (iter : I) -> Self { iter
                    .fold(Self::zero(), | a, & b | a + b) } } impl < V : Value <$s >>
                    Product for $name < V > { fn product < I : Iterator < Item = Self >>
                    (iter : I) -> Self { iter.fold(Self::one(), $name ::mul) } } impl
                    <'a, V : Value <$s >> Product <&'a $name < V >> for $name < V > { fn
                    product < I : Iterator < Item = &'a Self >> (iter : I) -> Self { iter
                    .fold(Self::one(), | a, & b | a * b) } } impl < V : Value <$s >>
                    BaseMod <$s > for $name < V > { fn val(self) -> $s { self.v } fn
                    modu() -> $s { V::val() } }
                };
            }
            mod_impl!(Modular, u32);
            mod_impl!(Modular64, u64);
            create_value!(V1e9_7, pub v1e9_7 : u32 = 1_000_000_007);
            pub type Mint1e9_7 = Modular<V1e9_7>;
            create_value!(V998244353, pub v998244353 : u32 = 998_244_353);
            pub type Mint998244353 = Modular<V998244353>;
        }
        pub mod num_traits {
            pub mod arithmetic {
                use std::{
                    fmt::{Debug, Display},
                    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
                };
                pub trait Arithmetic: Copy + Debug + Display + Add<
                        Output = Self,
                    > + AddAssign + Sub<
                        Output = Self,
                    > + SubAssign + Mul<
                        Output = Self,
                    > + MulAssign + PartialEq + PartialOrd {
                    fn zero() -> Self;
                    fn one() -> Self;
                    fn two() -> Self;
                    fn from_usize(val: usize) -> Self;
                    fn from_u8(val: u8) -> Self;
                }
                macro_rules! arithmetic_impl_float {
                    ($($t:ty)+) => {
                        $(impl Arithmetic for $t { fn zero() -> Self { 0.0 } fn one() ->
                        Self { 1.0 } fn two() -> Self { 2.0 } fn from_usize(val : usize)
                        -> Self { val as $t } fn from_u8(val : u8) -> Self { val as $t }
                        })+
                    };
                }
                arithmetic_impl_float!(f32 f64);
            }
            pub mod integer {
                use std::{
                    hash::Hash,
                    ops::{
                        BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign,
                        Div, DivAssign, Not, Rem, RemAssign, Shl, ShlAssign, Shr,
                        ShrAssign,
                    },
                };
                use crate::algo_lib::numeric::num_traits::arithmetic::Arithmetic;
                pub trait Integer: Arithmetic + Div<
                        Output = Self,
                    > + DivAssign + Rem<
                        Output = Self,
                    > + RemAssign + Shl<
                        Output = Self,
                    > + ShlAssign + Shr<
                        Output = Self,
                    > + ShrAssign + BitAnd<
                        Output = Self,
                    > + BitAndAssign + BitOr<
                        Output = Self,
                    > + BitOrAssign + BitXor<
                        Output = Self,
                    > + BitXorAssign + Not + Eq + Ord + Hash + 'static {
                    type Up: From<Self> + Integer;
                    fn max() -> Self;
                    fn min() -> Self;
                    fn downcast(val: Self::Up) -> Self;
                    fn upcast(self) -> Self::Up;
                    fn as_usize(self) -> usize;
                    fn gcd(self, rhs: Self) -> Self {
                        if rhs == Self::zero() { self } else { rhs.gcd(self % rhs) }
                    }
                    fn lcm(self, rhs: Self) -> Self {
                        self / self.gcd(rhs) * rhs
                    }
                    fn mod_mul(self, rhs: Self, m: Self) -> Self {
                        Self::downcast(self.upcast() * rhs.upcast() % m.upcast())
                    }
                    fn wrapping_add(self, rhs: Self) -> Self;
                    fn wrapping_sub(self, rhs: Self) -> Self;
                    fn wrapping_mul(self, rhs: Self) -> Self;
                    fn wrapping_div(self, rhs: Self) -> Self;
                }
                macro_rules! integer_impl {
                    ($t:ty, $up:ty) => {
                        impl Arithmetic for $t { fn zero() -> Self { 0 } fn one() -> Self
                        { 1 } fn two() -> Self { 2 } fn from_usize(val : usize) -> Self {
                        val as $t } fn from_u8(val : u8) -> Self { val as $t } } impl
                        Integer for $t { type Up = $up; fn max() -> Self { <$t >::MAX }
                        fn min() -> Self { <$t >::MIN } fn downcast(val : Self::Up) ->
                        Self { val as $t } fn upcast(self) -> Self::Up { self as $up } fn
                        as_usize(self) -> usize { self as usize } fn wrapping_add(self,
                        rhs : Self) -> Self { <$t >::wrapping_add(self, rhs) } fn
                        wrapping_sub(self, rhs : Self) -> Self { <$t
                        >::wrapping_sub(self, rhs) } fn wrapping_mul(self, rhs : Self) ->
                        Self { <$t >::wrapping_mul(self, rhs) } fn wrapping_div(self, rhs
                        : Self) -> Self { <$t >::wrapping_div(self, rhs) } }
                    };
                }
                integer_impl!(i128, i128);
                integer_impl!(i64, i128);
                integer_impl!(i32, i64);
                integer_impl!(i16, i32);
                integer_impl!(i8, i16);
                integer_impl!(isize, isize);
                integer_impl!(u128, u128);
                integer_impl!(u64, u128);
                integer_impl!(u32, u64);
                integer_impl!(u16, u32);
                integer_impl!(u8, u16);
                integer_impl!(usize, usize);
            }
            pub mod unsigned {
                use crate::algo_lib::numeric::num_traits::integer::Integer;
                pub trait Unsigned: Integer {}
                macro_rules! unsigned_impl {
                    ($($t:ty)+) => {
                        $(impl Unsigned for $t {})+
                    };
                }
                unsigned_impl!(usize u8 u16 u32 u64 u128);
                pub trait MUnsigned: Unsigned {}
                impl MUnsigned for u32 {}
                impl MUnsigned for u64 {}
            }
        }
    }
    pub mod util {
        pub mod value {
            use std::fmt::Debug;
            use std::hash::Hash;
            pub trait Value<
                T,
            >: Copy + Clone + PartialEq + Eq + PartialOrd + Ord + Hash + Debug + Default {
                fn val() -> T;
                fn set_val(v: T);
            }
            #[macro_export]
            macro_rules! create_value {
                ($name:ident, $v:vis $vname:ident : $t:ty = $val:literal) => {
                    #[allow(non_upper_case_globals)] static mut $vname : $t = $val;
                    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug,
                    Default)] pub struct $name; impl $crate::algo_lib::util::value::Value
                    <$t > for $name { fn val() -> $t { return unsafe { $vname }; } fn
                    set_val(v : $t) { unsafe { $vname = v; } } }
                };
            }
        }
    }
}
