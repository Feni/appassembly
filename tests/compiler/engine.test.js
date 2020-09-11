import { evaluate } from "@appassembly/compiler/engine.js"
import { CellEnv } from "@appassembly/compiler/CellEnv";

test('Evaluate basic expressions', () => {
    // When cells don't depend on each other, their order remains the same
    let byId = {
        0: {
            id: 0,
            name: "root",
            depends_on: [], 
        
            body: [1, 2, 3],
            params: []
        },
        1: {
            id: 1,
            name: "a",
            expr: "1 + 1",
            depends_on: [],
            
            body: [],
            params: []
        },
        2: {
            id: 2,
            name: "b",
            depends_on: [1],
            expr: "a * 2",
    
            body: [],
            params: []
        },
        3: {
            id: 3,
            name: "",
            depends_on: [2],
            expr: "b",
    
            body: [],
            params: []
        }
    }

    let output = evaluate(byId, 0).results;
    let outputDict = listToDict(output)

    expect(outputDict[1].value).toEqual(2)
    expect(outputDict[2].value).toEqual(4)
    expect(outputDict[3].value).toEqual(4)
});


test('Eval order of ops', () => {
    expect(evalExprs([{expr: "(2 + 3) * 4"}])[0].value).toEqual(20)
});

test('Eval multiple args', () => {
    expect(evalExprs([{
            name: "add", "expr": "(a, b): a + b"
        }, {
            expr: "add(4, 5)"
        }])[1].value).toEqual(9)
});

test('Attribute access', () => {
    let prog = [
        {   name: "movie", expr: 'title: "The Martian"\nyear: 2015' }, 
        {   expr: "movie.year"  },
        {   expr: "movie.title"  }
    ]
    let result = evalExprs(prog);
    expect(result[1].value).toEqual(2015)
    expect(result[2].value).toEqual("The Martian")

    // Accessing unknown attribute
    // Expressions on attribute access
});


test('Array indexing', () => {
    let prog = [
        {   name: "arr", expr: '[0, 1, 2, 3, 4, 5]' }, 
        {   expr: "arr[0]"  },
        {   expr: "arr[2]"  },
        {   expr: "arr[5]"  }
    ]
    let result = evalExprs(prog);
    expect(Array.from(result[0].value.iter())).toEqual([0, 1, 2, 3, 4, 5])
    expect(result[1].value).toEqual(0)
    expect(result[2].value).toEqual(2)
    expect(result[3].value).toEqual(5)

    // Index out of bounds.
    // Length check
});

test('Vector ops', () => {
    let prog = [
        {   name: "arr", expr: '[0, 1, 2, 3, 4, 5]' }, 
        {   expr: "arr + 5"  },
        {   expr: "arr > 2"  },
        {   expr: "arr == 3"  },
    ]
    let result = evalExprs(prog);
    expect(Array.from(result[0].value.iter())).toEqual([0, 1, 2, 3, 4, 5])
    expect(Array.from(result[1].value.iter())).toEqual([5, 6, 7, 8, 9, 10])
    expect(Array.from(result[2].value.iter())).toEqual([false, false, false, true, true, true])
    expect(Array.from(result[3].value.iter())).toEqual([false, false, false, true, false, false])


    prog = [
        {   name: "arr", expr: '[0, 1, 2, 3, 4, 5]' }, 
        {   expr: "arr > 2 == false"  },
        {   expr: "not (arr > 2)"  },
    ]
    result = evalExprs(prog);
    expect(Array.from(result[0].value.iter())).toEqual([0, 1, 2, 3, 4, 5])
    expect(Array.from(result[1].value.iter())).toEqual([true, true, true, false, false, false])
    expect(Array.from(result[2].value.iter())).toEqual([true, true, true, false, false, false])


    // Index out of bounds.
    // Length check
});



test('Array filtering', () => {
    let prog = [
        {   name: "arr", expr: '[9, 9, 10, 12, 15, 23]' }, 
        {   expr: "arr <= 10"  },
        {   expr: "arr[arr <= 10]"  },
    ]
    let result = evalExprs(prog);
    expect(Array.from(result[0].value.iter())).toEqual([9, 9, 10, 12, 15, 23])
    expect(Array.from(result[1].value.iter())).toEqual([true, true, true, false, false, false])
    expect(Array.from(result[2].value.iter())).toEqual([9, 9, 10])
});




test('Conditional guards', () => {
    let find_min = [
        {   name: "find_min",   expr: "(x, y) if(x <= y): x\n(x, y): y"  },
        {   expr: "find_min(5, 10)" },
        {   expr: "find_min(10, 5)" },
        {   expr: "find_min(5, 5)" },
    ]
    let result = evalExprs(find_min);
    expect(result[1].value).toEqual(5)
    expect(result[2].value).toEqual(5)
    expect(result[3].value).toEqual(5)
});

test('Conditions', () => {
    let find_min2 = [
        {   name: "find_min2",   expr: `
(x, y):
    if(x < y): x
    else if(x > y): y
    else: "Equal"`  },
        {   expr: "find_min2(5, 10)" },
        {   expr: "find_min2(30, 20)" },
        {   expr: "find_min2(42, 42)" },
    ]
    debugExprs(find_min2);
    let result = evalExprs(find_min2);
    expect(result[1].value).toEqual(5)
    expect(result[2].value).toEqual(20)
    expect(result[3].value).toEqual("Equal")
});


test('Generator ranges', () => {
    let prog = [
        { expr: '[1..10]' }, 
        { expr: '[1, 3 .. 10]' }, 
        { expr: '[5, 10 .. 100]' }, 
        { expr: '[100, 90, .., 10, 0]' }, 
    ]
    let result = evalExprs(prog);
    expect(Array.from(result[0].value.iter())).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(Array.from(result[1].value.iter())).toEqual([1, 3, 5, 7, 9])
    expect(Array.from(result[2].value.first(5))).toEqual([5, 10, 15, 20, 25])
    expect(Array.from(result[3].value.iter())).toEqual([100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 0])

});

test('Expr blocks', () => {
    let prog = [
        { name: "ex1", expr: '(x): \n\t1'},
        { name: "ex2", expr: '(x): \n\t1\n\t2'},
        { expr: 'ex1(0)'},
        { expr: 'ex2(0)'}
    ]

    let result = evalExprs(prog);
    expect(result[2].value).toEqual(1)
    expect(result[3].value).toEqual(2)
});


function listToDict(list, key="id") {
    let d = {};
    list.forEach((elem) => {
        d[elem[key]] = elem
    })
    return d
}

function debugExprs(exprs) {
    let byId = exprsToIdMap(exprs);
    let env = new CellEnv();
    env.create(byId, "root");
    env.parseAll("root");
    console.log("Root cell is : " + env.getCell("root"));
    env.debugAll(env.getCell("root"));
}

function exprsToIdMap(exprs) {
    let byId = {
        "root": {
            id: "root",
            name: "",
            expr: "",
            depends_on: [],
            
            body: [],
            params: []
        },
    }

    exprs.forEach((expr, index) => {
        byId.root.body.push(index)
        byId[index] = {
            id: index, 
            name: expr.name ? expr.name : "",
            expr: expr.expr, 
            depends_on: expr.depends_on ? expr.depends_on : [],
            body: expr.body ? expr.body : [],
            params: expr.params ? expr.params : []
        }
    })
    return byId;
}

function evalExprs(exprs) {
    let byId = exprsToIdMap(exprs);
    let output = evaluate(byId, "root").results;
    let outputDict = listToDict(output)
    return outputDict
}