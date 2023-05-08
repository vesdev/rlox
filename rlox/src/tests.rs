use crate::run;

macro_rules! check {
    ( $src:literal ) => {
        let src = indoc::indoc! {$src};
        println!("{}", src);

        if let Err(e) = run(src) {
            println!("{:#?}", e);
            panic!();
        }
    };
}

#[test]
fn loop_2d() {
    check! {r#"
        for(var i = 0; i < 3; i = i + 1)
        {
            for(var j = 0; j < 5; j = j + 1)
            {
                print j;
            }
        }
    "#};
}

#[test]
fn fib() {
    check! {r#"
    fun fib(n) {
        if (n < 2) return n;
        return fib(n - 2) + fib(n - 1);
    }
      
    var start = clock();
    print fib(20);
    print clock() - start;
    "#};
}

#[test]
fn upvalue_nested() {
    check! {r#"
    var inner;

    fun main() {
        var a = "Hello, outer!";
        fun outer() { 
            fun inner() { 
                a = "Hello, inner!";
                return a;
            }
            return inner;
        }
        inner = outer();
    }

    main();
    print inner();
    "#};
}

#[test]
fn upvalue_shared() {
    check! {r#"
    var A;
    var B;
    fun main() {
        var text = "Hello, outer!";
        fun a() { 
            print text;
        }
        fun b() { 
            print text;
        }
        A = a;
        B = b;
    }

    main();
    A();
    B();
    "#};
}
#[test]
fn upvalue_in_block() {
    check! {r#"
    var printA;

    fun main() {
        var a = "A initial";
        {
            var a = "A inner";
            var b = "B initial";
            fun inner() { 
                print a;
                print b;
            }

            printA = inner;
        }
        a = "A changed";
    }

    main();
    printA();
    "#};
}

#[test]
fn upvalue_for_loop() {
    check! {r#"
    var globalOne;
    var globalTwo;

    fun main() {
        for (var a = 1; a <= 2; a = a + 1) {
            fun closure() {
                print a;
            }
            if (globalOne == nil) {
                globalOne = closure;
            } else {
                globalTwo = closure;
            }
        }
    }

    main();
    globalOne();
    globalTwo();
    "#};
}

#[test]
fn upvalue_while_loop() {
    check! {r#"
    var globalOne;
    var globalTwo;

    fun main() {
        var i = 2;
        while (i > 0) {
            fun closure() {
                print i;
            }
            if (globalOne == nil) {
                globalOne = closure;
            } else {
                globalTwo = closure;
            }
            i = i - 1;
        }
    }

    main();
    globalOne();
    globalTwo();
    "#};
}
