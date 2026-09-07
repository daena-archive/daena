export type FormulaAst =
  | { type: "number"; value: number }
  | { type: "ref"; id: string }
  | { type: "unary"; op: "-"; arg: FormulaAst }
  | { type: "binary"; op: "+" | "-" | "*" | "/"; left: FormulaAst; right: FormulaAst }
  | { type: "call"; name: FormulaFunction; args: FormulaAst[] };

export type FormulaFunction = "min" | "max" | "avg" | "floor" | "round";

type Token =
  | { type: "number"; value: number }
  | { type: "ident"; value: string }
  | { type: "ref"; value: string }
  | { type: "symbol"; value: string }
  | { type: "end" };

const FUNCTIONS = new Set<string>(["min", "max", "avg", "floor", "round"]);
const UNARY_FUNCTIONS = new Set<string>(["floor", "round"]);

function isSymbol(token: Token, ...values: string[]): boolean {
  return token.type === "symbol" && values.includes(token.value);
}

export function parseFormula(source: string): { ast: FormulaAst; dependencies: string[] } | { error: string } {
  const trimmed = source.trim();
  if (!trimmed) return { error: "needs a formula" };
  try {
    const tokens = tokenize(trimmed);
    let index = 0;
    const peek = () => tokens[index] ?? { type: "end" as const };
    const take = () => tokens[index++] ?? { type: "end" as const };
    const ast = parseAdd();
    if (peek().type !== "end") throw new Error("unexpected input");
    const dependencies = [...collectRefs(ast)];
    return { ast, dependencies };

    function parseAdd(): FormulaAst {
      let left = parseMul();
      while (isSymbol(peek(), "+", "-")) {
        const op = (take() as { value: "+" | "-" }).value;
        left = { type: "binary", op, left, right: parseMul() };
      }
      return left;
    }

    function parseMul(): FormulaAst {
      let left = parseUnary();
      while (isSymbol(peek(), "*", "/")) {
        const op = (take() as { value: "*" | "/" }).value;
        left = { type: "binary", op, left, right: parseUnary() };
      }
      return left;
    }

    function parseUnary(): FormulaAst {
      if (isSymbol(peek(), "-")) {
        take();
        return { type: "unary", op: "-", arg: parseUnary() };
      }
      return parsePrimary();
    }

    function parsePrimary(): FormulaAst {
      const token = take();
      if (token.type === "number") return { type: "number", value: token.value };
      if (token.type === "ref") return { type: "ref", id: token.value };
      if (token.type === "ident") {
        if (!FUNCTIONS.has(token.value)) throw new Error("unknown function");
        if (!isSymbol(peek(), "(")) throw new Error("expected (");
        take();
        const args: FormulaAst[] = [];
        if (!isSymbol(peek(), ")")) {
          args.push(parseAdd());
          while (isSymbol(peek(), ",")) {
            take();
            args.push(parseAdd());
          }
        }
        if (!isSymbol(peek(), ")")) throw new Error("expected )");
        take();
        if (UNARY_FUNCTIONS.has(token.value) && args.length !== 1) throw new Error(`${token.value} takes one value`);
        if (!UNARY_FUNCTIONS.has(token.value) && args.length < 1) throw new Error(`${token.value} needs values`);
        return { type: "call", name: token.value as FormulaFunction, args };
      }
      if (isSymbol(token, "(")) {
        const inner = parseAdd();
        if (!isSymbol(peek(), ")")) throw new Error("expected )");
        take();
        return inner;
      }
      throw new Error("invalid formula");
    }
  } catch (cause) {
    return { error: cause instanceof Error ? cause.message : "invalid formula" };
  }
}

export function evaluateFormula(ast: FormulaAst, resolve: (id: string) => number | null): number | null {
  switch (ast.type) {
    case "number":
      return ast.value;
    case "ref":
      return resolve(ast.id);
    case "unary": {
      const arg = evaluateFormula(ast.arg, resolve);
      return arg === null ? null : -arg;
    }
    case "binary": {
      const left = evaluateFormula(ast.left, resolve);
      const right = evaluateFormula(ast.right, resolve);
      if (left === null || right === null) return null;
      if (ast.op === "+") return left + right;
      if (ast.op === "-") return left - right;
      if (ast.op === "*") return left * right;
      if (right === 0) return null;
      return left / right;
    }
    case "call": {
      const args: number[] = [];
      for (const arg of ast.args) {
        const value = evaluateFormula(arg, resolve);
        if (value === null) return null;
        args.push(value);
      }
      if (ast.name === "floor") return Math.floor(args[0] ?? 0);
      if (ast.name === "round") return Math.round(args[0] ?? 0);
      if (ast.name === "min") return Math.min(...args);
      if (ast.name === "max") return Math.max(...args);
      return args.reduce((sum, value) => sum + value, 0) / args.length;
    }
  }
}

export function formulaHasCycle(nodes: readonly { id: string; dependencies: string[] }[]): boolean {
  const derived = new Set(nodes.map((node) => node.id));
  const edges = new Map(nodes.map((node) => [node.id, node.dependencies.filter((id) => derived.has(id))]));
  const visiting = new Set<string>();
  const seen = new Set<string>();
  const visit = (id: string): boolean => {
    if (visiting.has(id)) return true;
    if (seen.has(id)) return false;
    visiting.add(id);
    for (const dep of edges.get(id) ?? []) {
      if (visit(dep)) return true;
    }
    visiting.delete(id);
    seen.add(id);
    return false;
  };
  return nodes.some((node) => visit(node.id));
}

function collectRefs(ast: FormulaAst, into = new Set<string>()): Set<string> {
  if (ast.type === "ref") into.add(ast.id);
  else if (ast.type === "unary") collectRefs(ast.arg, into);
  else if (ast.type === "binary") {
    collectRefs(ast.left, into);
    collectRefs(ast.right, into);
  } else if (ast.type === "call") {
    for (const arg of ast.args) collectRefs(arg, into);
  }
  return into;
}

function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let index = 0;
  while (index < source.length) {
    const ch = source[index];
    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      index += 1;
      continue;
    }
    if (ch === "{") {
      const end = source.indexOf("}", index + 1);
      if (end < 0) throw new Error("unclosed {");
      const id = source.slice(index + 1, end).trim();
      if (!id) throw new Error("empty component ref");
      if (id.includes("{") || id.includes("}")) throw new Error("invalid component ref");
      tokens.push({ type: "ref", value: id });
      index = end + 1;
      continue;
    }
    if (ch === "." || (ch >= "0" && ch <= "9")) {
      const start = index;
      if (ch === ".") {
        if (!(source[index + 1] >= "0" && source[index + 1] <= "9")) throw new Error("invalid number");
        index += 1;
      } else {
        while (index < source.length && source[index] >= "0" && source[index] <= "9") index += 1;
        if (source[index] === ".") {
          index += 1;
          while (index < source.length && source[index] >= "0" && source[index] <= "9") index += 1;
        }
      }
      while (index < source.length && source[index] >= "0" && source[index] <= "9") index += 1;
      tokens.push({ type: "number", value: Number(source.slice(start, index)) });
      continue;
    }
    if ((ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z")) {
      const start = index;
      index += 1;
      while (
        index < source.length &&
        ((source[index] >= "a" && source[index] <= "z") ||
          (source[index] >= "A" && source[index] <= "Z") ||
          (source[index] >= "0" && source[index] <= "9"))
      ) {
        index += 1;
      }
      tokens.push({ type: "ident", value: source.slice(start, index) });
      continue;
    }
    if ("+-*/(),".includes(ch)) {
      tokens.push({ type: "symbol", value: ch });
      index += 1;
      continue;
    }
    throw new Error("invalid formula");
  }
  tokens.push({ type: "end" });
  return tokens;
}
