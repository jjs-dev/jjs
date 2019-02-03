templates = {'page/login': '<p>\n        Log in, please:\n    <form action="/authenticate" method="post">\n        <label for="login">login</label>\n        <input type="text" name="login"/>\n        <br/>\n\n        <label for="password">password</label>\n        <input type="password" name="password">\n        <br/>\n\n        <input type="submit" title="Log in">\n    </form>\n    </p>', 'screen_name/login': 'log in', 'page/index': '', 'screen_name/index': 'index', 'screen_name/submit': 'submit', 'page/submit': '<form action="/submit" method="post" enctype="multipart/form-data">\n        <label>\n            toolchain: <input type="text" name="toolchain"/>\n        </label>\n        <br/>\n        <label>\n            file: <input type="file" name="code"/>\n        </label>\n        <br/>\n        <input type="submit" title="submit">\n    </form>'};

var orig_page = (document.location.pathname == '/')?'index':document.location.pathname.substr(1);

function loadPage(what, not_push)
{
    if(templates['screen_name/'+what] !== undefined && templates['page/'+what] !== undefined)
    {
        if(!not_push)
            window.history.pushState({"page": what}, "", "/"+(what=="index"?"":what));
        document.getElementById('page').innerHTML = templates['page/'+what];
        document.title = 'JJS: '+templates['screen_name/'+what];
        return false;
    }
    else if(not_push)
        document.location.replace('/'+what);
    else
        document.location.href = '/'+what;
}

window.onpopstate = function(e)
{
    if(e.state)
        loadPage(e.state.page, true);
    else
        loadPage(orig_page, true);
}
