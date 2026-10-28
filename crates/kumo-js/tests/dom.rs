mod common;

use common::{PAGE, eval, run, runtime};

#[test]
fn finds_elements_and_reads_properties() {
    assert_eq!(run("document.getElementById('p').textContent"), "hello");
    assert_eq!(run("document.getElementById('nope')"), "null");
    assert_eq!(run("document.querySelector('div#a.big').id"), "a");
    assert_eq!(run("document.querySelectorAll('li').length"), "3");
    assert_eq!(
        run("document.getElementsByTagName('li')[2].textContent"),
        "3"
    );
    assert_eq!(run("document.getElementsByClassName('box').length"), "1");
    assert_eq!(run("document.title"), "Test Page", "題名は空白をまとめる");
    assert_eq!(
        run("document.body.tagName + ',' + document.documentElement.nodeName"),
        "BODY,HTML"
    );
    assert_eq!(run("document.getElementById('a').className"), "box big");
    assert_eq!(
        run("document.getElementById('s').nodeType + ',' + document.nodeType"),
        "1,9"
    );
    assert_eq!(run("document.getElementById('in').value"), "v");
    assert_eq!(
        run("document.getElementById('link').href"),
        "https://example.com/next?x=1",
        "href は絶対 URL"
    );
    assert_eq!(
        run("document.getElementById('link').getAttribute('href')"),
        "/next?x=1"
    );
    assert_eq!(run("document.getElementById('a').dataset.fooBar"), "1");
    assert_eq!(run("String(document.querySelector('.none'))"), "null");
}

#[test]
fn selectors_matches_closest_and_errors() {
    assert_eq!(
        run("document.getElementById('s').matches('div > span')"),
        "true"
    );
    assert_eq!(run("document.getElementById('s').closest('.box').id"), "a");
    assert_eq!(run("document.getElementById('s').closest('ul')"), "null");
    assert_eq!(
        run("try { document.querySelector('###'); 'no error' } catch (e) { e.name }"),
        "SyntaxError"
    );
}

#[test]
fn tree_navigation() {
    assert_eq!(run("document.getElementById('p').parentNode.id"), "a");
    assert_eq!(run("document.getElementById('p').nextSibling.id"), "s");
    assert_eq!(
        run("document.getElementById('s').previousElementSibling.id"),
        "p"
    );
    assert_eq!(run("document.getElementById('a').children.length"), "2");
    assert_eq!(
        run("document.getElementById('list').firstElementChild.textContent"),
        "1"
    );
    assert_eq!(
        run("document.getElementById('list').lastChild.textContent"),
        "3"
    );
    assert_eq!(
        run("document.getElementById('list').childNodes.length"),
        "3"
    );
    assert_eq!(
        run("document.getElementById('p').ownerDocument === document"),
        "true"
    );
    assert_eq!(
        run("document.documentElement.parentNode === document"),
        "true"
    );
    assert_eq!(
        run("document.getElementById('a').contains(document.getElementById('s'))"),
        "true"
    );
    assert_eq!(
        run("document.getElementById('s').contains(document.getElementById('a'))"),
        "false"
    );
    assert_eq!(
        run("document.getElementById('a') === document.getElementById('a')"),
        "true",
        "同じノードは同じオブジェクト"
    );
}

#[test]
fn mutates_the_shared_dom_and_marks_it_dirty() {
    let (mut js, doc) = runtime(PAGE);
    assert!(!js.take_dirty());
    eval(
        &mut js,
        "document.getElementById('p').textContent = 'changed'",
    );
    assert!(js.take_dirty(), "変更したら印が立つ");
    assert!(!js.take_dirty(), "取り出すと戻る");
    assert!(
        doc.borrow()
            .text_content(doc.borrow().root())
            .contains("changed")
    );

    eval(
        &mut js,
        "const e = document.createElement('B'); e.id = 'new'; e.textContent = 'bold';
         document.getElementById('a').appendChild(e);
         document.getElementById('list').insertBefore(document.createElement('li'), document.getElementById('list').firstChild);
         document.getElementById('s').remove();",
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('a').innerHTML"),
        "<p id=\"p\">changed</p><b id=\"new\">bold</b>"
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('list').children.length"),
        "4"
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('s')"),
        "null",
        "remove した要素は検索に出ない"
    );
    assert!(
        doc.borrow()
            .descendants(doc.borrow().root())
            .any(|n| doc.borrow().attr(n, "id") == Some("new"))
    );
}

#[test]
fn attributes_class_list_style_and_data() {
    let (mut js, doc) = runtime(PAGE);
    eval(
        &mut js,
        "const a = document.getElementById('a');
         a.setAttribute('Title', 'T'); a.classList.add('x', 'y'); a.classList.remove('box'); a.classList.toggle('big');
         a.style.backgroundColor = 'red'; a.style.display = 'none'; a.style.setProperty('margin-top', '3px');
         a.dataset.newKey = 'v'; a.hidden = true;",
    );
    let a = doc
        .borrow()
        .descendants(doc.borrow().root())
        .find(|&n| doc.borrow().attr(n, "id") == Some("a"))
        .unwrap();
    let d = doc.borrow();
    assert_eq!(d.attr(a, "title"), Some("T"));
    assert_eq!(d.attr(a, "class"), Some("x y"));
    assert_eq!(
        d.attr(a, "style"),
        Some("background-color: red; display: none; margin-top: 3px;")
    );
    assert_eq!(d.attr(a, "data-new-key"), Some("v"));
    assert!(d.attr(a, "hidden").is_some());
    drop(d);
    assert_eq!(
        eval(
            &mut js,
            "document.getElementById('a').style.backgroundColor"
        ),
        "red"
    );
    assert_eq!(
        eval(
            &mut js,
            "document.getElementById('a').classList.contains('y')"
        ),
        "true"
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('a').classList.length"),
        "2"
    );
    eval(
        &mut js,
        "document.getElementById('a').style.display = ''; delete document.getElementById('a').dataset.newKey",
    );
    assert_eq!(
        eval(
            &mut js,
            "document.getElementById('a').style.display + '|' + document.getElementById('a').dataset.newKey"
        ),
        "|undefined"
    );
    assert_eq!(
        eval(
            &mut js,
            "document.getElementById('a').getAttributeNames().join()"
        ),
        "id,class,data-foo-bar,title,style,hidden"
    );
}

#[test]
fn inner_html_text_nodes_and_fragments() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "document.getElementById('list').innerHTML = '<li class=\"n\">a &amp; b</li><li>c</li><script>1</script>'",
    );
    assert_eq!(
        eval(&mut js, "document.querySelectorAll('#list li').length"),
        "2"
    );
    assert_eq!(
        eval(&mut js, "document.querySelector('#list .n').textContent"),
        "a & b"
    );
    assert!(
        js.take_new_scripts().is_empty(),
        "innerHTML のスクリプトは実行対象にならない"
    );
    eval(
        &mut js,
        "const f = document.createDocumentFragment(); f.appendChild(document.createTextNode('t1'));
         const b = document.createElement('b'); b.textContent = 'B'; f.appendChild(b);
         document.getElementById('s').appendChild(f);
         document.getElementById('p').insertAdjacentHTML('afterend', '<i>I</i>');
         document.getElementById('p').before('text', document.createElement('hr'));",
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('a').innerHTML"),
        "text<hr><p id=\"p\">hello</p><i>I</i><span id=\"s\">xt1<b>B</b></span>"
    );
    assert_eq!(
        eval(
            &mut js,
            "document.getElementById('p').firstChild.data + document.getElementById('p').firstChild.length"
        ),
        "hello5"
    );
    assert_eq!(
        eval(
            &mut js,
            "const c = document.getElementById('a').cloneNode(true); c.id = 'c'; c.querySelectorAll('*').length"
        ),
        "5"
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('c')"),
        "null",
        "複製は文書に入っていない"
    );
}

#[test]
fn rejects_cycles_and_wrong_arguments() {
    assert_eq!(
        run(
            "try { const a = document.getElementById('a'); document.getElementById('p').appendChild(a); 'ok' } catch (e) { e.name }"
        ),
        "TypeError"
    );
    assert_eq!(
        run("try { document.body.appendChild({}); 'ok' } catch (e) { e.name }"),
        "TypeError"
    );
    assert_eq!(
        run(
            "try { document.getElementById('a').removeChild(document.getElementById('list')); 'ok' } catch (e) { e.name }"
        ),
        "TypeError"
    );
}

#[test]
fn form_values_and_reflected_properties() {
    let (mut js, _) = runtime(
        "<form id=f><input id=i name=q value=a><input id=c type=checkbox><textarea id=t>base</textarea>\
         <select id=s><option value=x>X<option value=y selected>Y</select></form>",
    );
    let vars = "const [f, i, c, t, s] = ['f', 'i', 'c', 't', 's'].map((id) => document.getElementById(id));";
    let wrap = |body: &str| format!("(() => {{ {vars} {body} }})()");
    let show = wrap("return i.value + ',' + t.value + ',' + s.value + ',' + c.checked");
    assert_eq!(eval(&mut js, &show), "a,base,y,false");
    eval(
        &mut js,
        &wrap("i.value = 'b'; c.checked = true; t.value = 'T'; s.value = 'x'"),
    );
    assert_eq!(eval(&mut js, &show), "b,T,x,true");
    assert_eq!(
        eval(
            &mut js,
            &wrap(
                "return i.name + ',' + i.type + ',' + f.querySelector('option[selected]').textContent"
            )
        ),
        "q,,X"
    );
}

#[test]
fn reinserting_in_place_is_harmless() {
    let (mut js, _) = runtime(PAGE);
    eval(
        &mut js,
        "const a = document.getElementById('a'); a.prepend(a.firstChild); a.insertBefore(a.firstChild, a.firstChild);
         a.replaceWith(a); const l = document.getElementById('list'); l.after(l); l.before(l);",
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('a').children.length"),
        "2"
    );
    assert_eq!(
        eval(&mut js, "document.getElementById('list').children.length"),
        "3"
    );
}

#[test]
fn legacy_web_apis_are_available() {
    // Annex B の関数（古いスクリプトが広く使う）と、`new Image()` / `new Option()`
    assert_eq!(
        run("'hello'.substr(1, 3) + escape('a b') + unescape('%41') + 'x'.anchor('n')"),
        "ell".to_owned() + "a%20bA" + "<a name=\"n\">x</a>"
    );
    assert_eq!(
        run(
            "const o = {}; o.__proto__ = {k: 1}; String(o.k) + ('a'.trimLeft === String.prototype.trimStart)"
        ),
        "1true"
    );
    assert_eq!(
        run(
            "const i = new Image(3, 4); [i.tagName, i.getAttribute('width'), i.getAttribute('height'), new Option('T', 'v', false, true).outerHTML].join()"
        ),
        "IMG,3,4,<option value=\"v\" selected=\"\">T</option>"
    );
}
