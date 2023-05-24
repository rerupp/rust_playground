use crate::foo::Struct;
use std::sync::Once;
use trace::*;

// initialize the logging environment 1 time
static INIT: Once = Once::new();
fn initialize() {
    INIT.call_once(|| {
        let mut builder = env_logger::builder();
        builder.target(env_logger::Target::Stdout).filter_level(log::LevelFilter::Trace);
        let _ = builder.try_init();
    });
}

#[trace]
fn standalone() {}

#[test]
fn test_standalone() {
    initialize();
    standalone();
}

mod foo {
    use trace::*;
    pub struct Struct {
        pub salutation: String,
    }
    impl Struct {
        #[trace]
        pub fn greet(&self) {
            eprintln!("struct instance says '{}'!!!", self.salutation);
        }
        #[trace]
        pub fn struct_greet(salutation: &str) {
            eprintln!("struct fn says '{salutation}'!!!");
        }
    }
}

#[test]
fn struct_member() {
    initialize();
    let data = foo::Struct { salutation: String::from("Hello there") };
    data.greet();
    Struct::struct_greet("Ugh, Hi")
}

mod experiment {
    use std::any::{Any, TypeId};

    fn type_id<T: ?Sized + Any>(_: &T) -> &'static str {
        std::any::type_name::<T>()
    }
    struct TestCase;
    impl TestCase {
        fn type_id(&self) {
            log::trace!("self = {:?}", type_id(self));
        }
    }

    fn is_string<T: ?Sized + Any>(_s: &T) -> bool {
        TypeId::of::<String>() == TypeId::of::<T>()
    }

    #[test]
    fn example() {
        super::initialize();
        assert_eq!(is_string(&0), false);
        assert_eq!(is_string(&"cookie monster".to_string()), true);
        let testcase = TestCase {};
        testcase.type_id();
    }
}
