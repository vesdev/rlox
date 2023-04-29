use crate::run;

#[test]
fn loop_2d() {
    let src = indoc::indoc! {r#"

        for(var i = 0; i < 3; i = i + 1)
        {
            for(var j = 0; j < 5; j = j + 1)
            {
                print j;
            }
        }
    "#};

    println!("{}", src);

    if let Err(e) = run(src) {
        println!("{:#?}", e);
        panic!();
    }
}

#[test]
fn fib() {
    let src = indoc::indoc! {r#"
    fun fib(n) {
        if (n < 2) return n;
        return fib(n - 2) + fib(n - 1);
    }
      
    var start = clock();
    print fib(30);
    print clock() - start;

    "#};

    println!("{}", src);

    if let Err(e) = run(src) {
        println!("{:#?}", e);
        panic!();
    }
}
