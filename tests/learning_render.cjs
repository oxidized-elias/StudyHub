const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const ctx={APP_BASE:'',marked:require(process.env.MARKED_RUNTIME||'marked'),katex:require(process.env.KATEX_RUNTIME||'katex'),DOMPurify:{sanitize:s=>s},mermaid:{initialize(){}}};vm.createContext(ctx);
const html=fs.readFileSync('apps/SelfHosted-AI_Tests/web/index.html','utf8');const source=html.split('mermaid.initialize(')[1].split('\nfunction Md')[0];vm.runInContext('mermaid.initialize('+source+'\nglobalThis.renderMarkdown=render;',ctx);
const render=s=>ctx.renderMarkdown(s,0,true);
for(const expression of ['f(x)=a_n x^n + \x08dots + a_0','g(x)=\x0crac{1}{x}','\theta+\t'+'imes x','\r'+'ightarrow','\n'+'abla f']){const output=render('$'+expression+'$');assert.match(output,/class="katex"/);assert.doesNotMatch(output,/katex-error|math-fallback/)}
let output=render('Definition [S2.1, S2.2]. Example [S2.1].');assert.equal((output.match(/data-source=/g)||[]).length,3);assert.match(output,/data-source="2.2"/);
output=render('`$literal$`\n\n```text\n$not math$\n```');assert.doesNotMatch(output,/class="katex"/);
output=render('Related [[Polynomial Functions]]');assert.match(output,/data-note="Polynomial Functions"/);
output=render('f(x) = -2x^5 + 3x^2 - 1');assert.match(output,/class="katex"/);assert.match(output,/x\^\{5\}/);assert.match(output,/x\^\{2\}/);
output=render('x^12');assert.match(output,/class="katex"/);assert.match(output,/x\^\{12\}/);
output=render('```text\nf(x) = -2x^5 + 3x^2 - 1\n```');assert.doesNotMatch(output,/class="katex"/);assert.match(output,/x\^5/);
output=render('Use your notes to explain why.');assert.doesNotMatch(output,/class="katex"/);
console.log('PASS: persisted LaTeX control-character repair, grouped section citations, literal code protection, and connected-note links.');
