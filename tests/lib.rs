#![feature(path_relative_from)]

use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Compares a generated header file to an expected one using `cmp_header.py`.
///
/// Do not put any boiler plate in the strings, only put the items you want to test.
///
/// For the rust file omit:
///
/// ```none
/// #![feature(plugin)]
/// #![plugin(cheddar)]
/// ```
///
/// For the header file omit:
///
/// ```none
/// #ifndef cheddar_gen_cheddar_h
/// #define cheddar_gen_cheddar_h
///
/// #ifdef __cplusplus
/// extern "C" {
/// #endif
///
/// #include <stdint.h>
/// #include <stdbool.h>
///
/// #ifdef __cplusplus
/// }
/// #endif
///
/// #endif
/// ```
macro_rules! cheddar_cmp_test {
    ($name:ident, $header:expr, $rust:expr) => {
        #[test]
        fn $name() {
            let package_dir = std::env::current_dir()
                .expect("internal testing error: unable to find current directory");
            let test_dir = package_dir.join("tests");
            let dir = test_dir.join(stringify!($name));
            let plugin_dir = package_dir
                .join("target")
                .join("debug");
            let cmp_script = test_dir.join("cmp_header.py");
            let source = dir.join(concat!(stringify!($name), ".rs"));
            let expected_header = dir.join("expected.h");
            let actual_header = dir.join("actual.h");

            // Create and move into a fresh directory.
            std::fs::create_dir_all(&dir)
                .expect("internal testing error: could not create directory");

            // Write the expected header and source files.
            File::create(&expected_header)
                .expect("internal testing error: could not create expected header file")
                .write_all(concat!(
                    // Due to the way CppHeaderParser works we only need to add the #define.
                    "#define cheddar_gen_actual_h\n",
                    $header,
                ).as_bytes())
                .expect("internal testing error: could not write to expected header file");

            let dir_rel_to_cur = dir.relative_from(&package_dir)
                .expect("internal testing error: unable determine relative path to test dir")
                .iter();

            let mut cheddar_args = String::new();
            for path in dir_rel_to_cur {
                cheddar_args.push_str(path.to_str()
                    .expect("internal testing error: can not handle non-utf8 file paths")
                );
                cheddar_args.push_str(", ");
            }

            File::create(&source)
                .expect("internal testing error: could not create rust source file")
                .write_all(format!(
                    "#![feature(plugin)]\n#![plugin(cheddar({}actual))]\n{}",
                    cheddar_args, $rust,
                ).as_bytes())
                .expect("internal testing error: could not write to rust source file");

            // Compile the header.
            let output = Command::new("rustc")
                .args(&["--crate-type", "dylib"])
                .arg("-L").arg(plugin_dir.as_os_str())
                .args(&["-Z", "no-trans"])
                .arg(&source)
                .output()
                .expect("internal testing error: could not run `rustc`");

            if !output.status.success() { panic!(
                "internal testing error: compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ); }

            // Compare the headers.
            let output = Command::new(&cmp_script)
                .arg(&expected_header)
                .arg(&actual_header)
                .output()
                .expect("internal testing error: could not run `cmp_header.py`");

            if !output.status.success() {
                if !output.stderr.is_empty() {
                    panic!(
                        "internal testing error: `cmp_header.py` failed: {}",
                        String::from_utf8_lossy(&output.stderr),
                    );
                } else {
                    panic!("{}", String::from_utf8_lossy(&output.stdout));
                }
            }
        }
    };
}



cheddar_cmp_test! { test_compilable_typedefs,
    "
    typedef int64_t Int64;
    ",
    "
    pub type Int64 = i64;
    // Shouldn't appear in output header file.
    type Float32 = f32;
    // Shouldn't appear in output header file.
    pub type AResult<T> = Result<T, ()>;
    "
}

cheddar_cmp_test! { test_compilable_enums,
    "
    typedef enum Colours {
        Red,
        Orange,
        Yellow,
        Green,
        Blue,
        Indigo,
        Violet,
    } Colours;

    typedef enum TypesOfLabrador {
        Stupid = -8,
        Braindead,
    } TypesOfLabrador;
    ",
    "
    #[repr(C)]
    pub enum Colours {
        Red,
        Orange,
        Yellow,
        Green,
        Blue,
        Indigo,
        Violet,
    }

    // Shouldn't appear in the output header file.
    #[allow(dead_code)]
    #[repr(C)]
    enum Planets {
        Mercury,
        Venus,
        Earth,
        Mars,
        Jupiter,
        Saturn,
        Neptune,
    }

    // Shouldn't appear in the output header file.
    pub enum Days {
        Sunday,
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday,
    }

    #[repr(C)]
    pub enum TypesOfLabrador {
        Stupid = -8,
        Braindead,
    }
    "
}

cheddar_cmp_test! { test_compilable_structs,
    "
    typedef struct Student {
        int32_t id;
        int32_t roll;
        double score;
    } Student;
    ",
    "
    #[repr(C)]
    pub struct Student {
        id: i32,
        roll: i32,
        score: f64,
    }

    // Shouldn't appear in the output header file.
    #[allow(dead_code)]
    #[repr(C)]
    struct Complex {
        real: f64,
        imag: f64,
    }

    // Shouldn't appear in output header file.
    pub struct Employee {
        id: i32,
        age: i16,
        salary: f64,
    }
    "
}

cheddar_cmp_test! { test_compilable_functions,
    "
    int64_t add_i64(int64_t lhs, int64_t rhs);
    ",
    r#"
    #[no_mangle]
    pub extern fn add_i64(lhs: i64, rhs: i64) -> i64 {
        lhs + rhs
    }

    // Shouldn't appear in the output header file.
    #[allow(dead_code)]
    #[no_mangle]
    extern fn add_i32(lhs: i32, rhs: i32) -> i32 {
        lhs + rhs
    }

    // Shouldn't appear in output header file.
    #[no_mangle]
    pub fn add_i16(lhs: i16, rhs: i16) -> i16 {
        lhs + rhs
    }

    // Shouldn't appear in output header file.
    pub fn add_i8(lhs: i8, rhs: i8) -> i8 {
        lhs + rhs
    }
    "#
}

cheddar_cmp_test! { test_pure_rust_types,
    "
    typedef void MyVoid;
    typedef float Float32;
    typedef double Float64;
    typedef int8_t Int8;
    typedef int16_t Int16;
    typedef int32_t Int32;
    typedef int64_t Int64;
    typedef intptr_t Int;
    typedef uint8_t UInt8;
    typedef uint16_t UInt16;
    typedef uint32_t UInt32;
    typedef uint64_t UInt64;
    typedef uintptr_t UInt;
    typedef bool Bool;
    typedef double* FloatArray;
    typedef const bool* LogicArray;
    typedef int32_t**** FourPointers;
    ",
    "
    pub type MyVoid = ();
    pub type Float32 = f32;
    pub type Float64 = f64;
    pub type Int8 = i8;
    pub type Int16 = i16;
    pub type Int32 = i32;
    pub type Int64 = i64;
    pub type Int = isize;
    pub type UInt8 = u8;
    pub type UInt16 = u16;
    pub type UInt32 = u32;
    pub type UInt64 = u64;
    pub type UInt = usize;
    pub type Bool = bool;
    pub type FloatArray = *mut f64;
    pub type LogicArray = *const bool;
    pub type FourPointers = *mut *mut *mut *mut i32;
    "
}

cheddar_cmp_test! { test_general_interplay,
    "
    typedef float Kg;
    typedef float Lbs;
    typedef float M;
    typedef float Ins;

    typedef enum Eye {
        Blue = -1,
        Green,
        Red,
    } Eye;

    typedef struct Person {
        int8_t age;
        Eye eyes;
        Kg weight;
        M height;
    } Person;

    Person Person_create(int8_t age, Eye eyes, float weight_lbs, float height_ins);
    void Person_describe(Person person);
    ",
    r#"
    pub type Kg = f32;
    pub type Lbs = f32;
    pub type M = f32;
    pub type Ins = f32;

    #[repr(C)]
    pub enum Eye {
        Blue = -1,
        Green,
        Red,
    }

    #[repr(C)]
    pub struct Person {
        age: i8,
        eyes: Eye,
        weight: Kg,
        height: M,
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern fn Person_create(age: i8, eyes: Eye, weight_lbs: f32, height_ins: f32) -> Person {
        Person {
            age: age,
            eyes: eyes,
            weight: weight_lbs * 0.45,
            height: height_ins * 0.0254,
        }
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern fn Person_describe(person: Person) {
        let eyes = match person.eyes {
            Eye::Blue => "blue",
            Eye::Green => "green",
            Eye::Red => "red",
        };
        println!(
            "The {}m {} year old weighed {}kg and had {} eyes.",
            person.height, person.age, person.weight, eyes,
        );
    }
    "#
}