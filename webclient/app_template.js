templates = TEMPLATES;

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
