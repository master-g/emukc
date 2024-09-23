var gadgets = gadgets || {};
var shindig = shindig || {};
var osapi = osapi || {};
gadgets.config = (function () {
  var components = {};
  var configuration;
  return {
    register: function (component, opt_validators, opt_callback) {
      var registered = components[component];
      if (!registered) {
        registered = [];
        components[component] = registered;
      }
      registered.push({
        validators: opt_validators || {},
        callback: opt_callback,
      });
    },
    get: function (opt_component) {
      if (opt_component) {
        return configuration[opt_component] || {};
      }
      return configuration;
    },
    init: function (config, opt_noValidation) {
      configuration = config;
      for (var name in components) {
        if (components.hasOwnProperty(name)) {
          var componentList = components[name],
            conf = config[name];
          for (var i = 0, j = componentList.length; i < j; ++i) {
            var component = componentList[i];
            if (conf && !opt_noValidation) {
              var validators = component.validators;
              for (var v in validators) {
                if (validators.hasOwnProperty(v)) {
                  if (!validators[v](conf[v])) {
                    throw new Error(
                      'Invalid config value "' +
                        conf[v] +
                        '" for parameter "' +
                        v +
                        '" in component "' +
                        name +
                        '"',
                    );
                  }
                }
              }
            }
            if (component.callback) {
              component.callback(config);
            }
          }
        }
      }
    },
    EnumValidator: function (list) {
      var listItems = [];
      if (arguments.length > 1) {
        for (var i = 0, arg; (arg = arguments[i]); ++i) {
          listItems.push(arg);
        }
      } else {
        listItems = list;
      }
      return function (data) {
        for (var i = 0, test; (test = listItems[i]); ++i) {
          if (data === listItems[i]) {
            return true;
          }
        }
        return false;
      };
    },
    RegExValidator: function (re) {
      return function (data) {
        return re.test(data);
      };
    },
    ExistsValidator: function (data) {
      return typeof data !== "undefined";
    },
    NonEmptyStringValidator: function (data) {
      return typeof data === "string" && data.length > 0;
    },
    BooleanValidator: function (data) {
      return typeof data === "boolean";
    },
    LikeValidator: function (test) {
      return function (data) {
        for (var member in test) {
          if (test.hasOwnProperty(member)) {
            var t = test[member];
            if (!t(data[member])) {
              return false;
            }
          }
        }
        return true;
      };
    },
  };
})();
gadgets.config.isGadget = false;
gadgets.config.isContainer = true;
if (window.JSON && window.JSON.parse && window.JSON.stringify) {
  gadgets["json"] = (function () {
    var endsWith___ = /___$/;
    return {
      parse: function (str) {
        try {
          return window.JSON.parse(str);
        } catch (e) {
          return false;
        }
      },
      stringify: function (obj) {
        try {
          return window.JSON.stringify(obj, function (k, v) {
            return !endsWith___.test(k) ? v : null;
          });
        } catch (e) {
          return null;
        }
      },
    };
  })();
} else {
  gadgets["json"] = (function () {
    function f(n) {
      return n < 10 ? "0" + n : n;
    }
    Date.prototype.toJSON = function () {
      return [
        this.getUTCFullYear(),
        "-",
        f(this.getUTCMonth() + 1),
        "-",
        f(this.getUTCDate()),
        "T",
        f(this.getUTCHours()),
        ":",
        f(this.getUTCMinutes()),
        ":",
        f(this.getUTCSeconds()),
        "Z",
      ].join("");
    };
    var m = {
      "\b": "\\b",
      "\t": "\\t",
      "\n": "\\n",
      "\f": "\\f",
      "\r": "\\r",
      '"': '\\"',
      "\\": "\\\\",
    };
    function stringify(value) {
      var a,
        i,
        k,
        l,
        r = /["\\\x00-\x1f\x7f-\x9f]/g,
        v;
      switch (typeof value) {
        case "string":
          return r.test(value)
            ? '"' +
                value.replace(r, function (a) {
                  var c = m[a];
                  if (c) {
                    return c;
                  }
                  c = a.charCodeAt();
                  return (
                    "\\u00" +
                    Math.floor(c / 16).toString(16) +
                    (c % 16).toString(16)
                  );
                }) +
                '"'
            : '"' + value + '"';
        case "number":
          return isFinite(value) ? String(value) : "null";
        case "boolean":
        case "null":
          return String(value);
        case "object":
          if (!value) {
            return "null";
          }
          a = [];
          if (
            typeof value.length === "number" &&
            !value.propertyIsEnumerable("length")
          ) {
            l = value.length;
            for (i = 0; i < l; i += 1) {
              a.push(stringify(value[i]) || "null");
            }
            return "[" + a.join(",") + "]";
          }
          for (k in value) {
            if (k.match("___$")) continue;
            if (value.hasOwnProperty(k)) {
              if (typeof k === "string") {
                v = stringify(value[k]);
                if (v) {
                  a.push(stringify(k) + ":" + v);
                }
              }
            }
          }
          return "{" + a.join(",") + "}";
      }
      return "undefined";
    }
    return {
      stringify: stringify,
      parse: function (text) {
        if (
          /^[\],:{}\s]*$/.test(
            text
              .replace(/\\["\\\/b-u]/g, "@")
              .replace(
                /"[^"\\\n\r]*"|true|false|null|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?/g,
                "]",
              )
              .replace(/(?:^|:|,)(?:\s*\[)+/g, ""),
          )
        ) {
          return eval("(" + text + ")");
        }
        return false;
      },
    };
  })();
}
gadgets["json"].flatten = function (obj) {
  var flat = {};
  if (obj === null || obj === undefined) return flat;
  for (var k in obj) {
    if (obj.hasOwnProperty(k)) {
      var value = obj[k];
      if (null === value || undefined === value) {
        continue;
      }
      flat[k] =
        typeof value === "string" ? value : gadgets.json.stringify(value);
    }
  }
  return flat;
};
var tamings___ = tamings___ || [];
tamings___.push(function (imports) {
  ___.tamesTo(gadgets.json.stringify, safeJSON.stringify);
  ___.tamesTo(gadgets.json.parse, safeJSON.parse);
});
gadgets["util"] = (function () {
  function parseUrlParams(url) {
    var query;
    var queryIdx = url.indexOf("?");
    var hashIdx = url.indexOf("#");
    if (hashIdx === -1) {
      query = url.substr(queryIdx + 1);
    } else {
      query = [
        url.substr(queryIdx + 1, hashIdx - queryIdx - 1),
        "&",
        url.substr(hashIdx + 1),
      ].join("");
    }
    return query.split("&");
  }
  var parameters = null;
  var features = {};
  var services = {};
  var onLoadHandlers = [];
  var escapeCodePoints = {
    0: false,
    10: true,
    13: true,
    34: true,
    39: true,
    60: true,
    62: true,
    92: true,
    8232: true,
    8233: true,
  };
  function unescapeEntity(match, value) {
    return String.fromCharCode(value);
  }
  function init(config) {
    features = config["core.util"] || {};
  }
  if (gadgets.config) {
    gadgets.config.register("core.util", null, init);
  }
  return {
    getUrlParameters: function (opt_url) {
      var no_opt_url = typeof opt_url === "undefined";
      if (parameters !== null && no_opt_url) {
        return parameters;
      }
      var parsed = {};
      var pairs = parseUrlParams(opt_url || document.location.href);
      var unesc = window.decodeURIComponent ? decodeURIComponent : unescape;
      for (var i = 0, j = pairs.length; i < j; ++i) {
        var pos = pairs[i].indexOf("=");
        if (pos === -1) {
          continue;
        }
        var argName = pairs[i].substring(0, pos);
        var value = pairs[i].substring(pos + 1);
        value = value.replace(/\+/g, " ");
        parsed[argName] = unesc(value);
      }
      if (no_opt_url) {
        parameters = parsed;
      }
      return parsed;
    },
    makeClosure: function (scope, callback, var_args) {
      var baseArgs = [];
      for (var i = 2, j = arguments.length; i < j; ++i) {
        baseArgs.push(arguments[i]);
      }
      return function () {
        var tmpArgs = baseArgs.slice();
        for (var i = 0, j = arguments.length; i < j; ++i) {
          tmpArgs.push(arguments[i]);
        }
        return callback.apply(scope, tmpArgs);
      };
    },
    makeEnum: function (values) {
      var i,
        v,
        obj = {};
      for (i = 0; (v = values[i]); ++i) {
        obj[v] = v;
      }
      return obj;
    },
    getFeatureParameters: function (feature) {
      return typeof features[feature] === "undefined"
        ? null
        : features[feature];
    },
    hasFeature: function (feature) {
      return typeof features[feature] !== "undefined";
    },
    getServices: function () {
      return services;
    },
    registerOnLoadHandler: function (callback) {
      onLoadHandlers.push(callback);
    },
    runOnLoadHandlers: function () {
      for (var i = 0, j = onLoadHandlers.length; i < j; ++i) {
        onLoadHandlers[i]();
      }
    },
    escape: function (input, opt_escapeObjects) {
      if (!input) {
        return input;
      } else if (typeof input === "string") {
        return gadgets.util.escapeString(input);
      } else if (typeof input === "array") {
        for (var i = 0, j = input.length; i < j; ++i) {
          input[i] = gadgets.util.escape(input[i]);
        }
      } else if (typeof input === "object" && opt_escapeObjects) {
        var newObject = {};
        for (var field in input) {
          if (input.hasOwnProperty(field)) {
            newObject[gadgets.util.escapeString(field)] = gadgets.util.escape(
              input[field],
              true,
            );
          }
        }
        return newObject;
      }
      return input;
    },
    escapeString: function (str) {
      if (!str) return str;
      var out = [],
        ch,
        shouldEscape;
      for (var i = 0, j = str.length; i < j; ++i) {
        ch = str.charCodeAt(i);
        shouldEscape = escapeCodePoints[ch];
        if (shouldEscape === true) {
          out.push("&#", ch, ";");
        } else if (shouldEscape !== false) {
          out.push(str.charAt(i));
        }
      }
      return out.join("");
    },
    unescapeString: function (str) {
      if (!str) return str;
      return str.replace(/&#([0-9]+);/g, unescapeEntity);
    },
    attachBrowserEvent: function (elem, eventName, callback, useCapture) {
      if (typeof elem.addEventListener != "undefined") {
        elem.addEventListener(eventName, callback, useCapture);
      } else if (typeof elem.attachEvent != "undefined") {
        elem.attachEvent("on" + eventName, callback);
      } else {
        gadgets.warn("cannot attachBrowserEvent: " + eventName);
      }
    },
    removeBrowserEvent: function (elem, eventName, callback, useCapture) {
      if (elem.removeEventListener) {
        elem.removeEventListener(eventName, callback, useCapture);
      } else if (elem.detachEvent) {
        elem.detachEvent("on" + eventName, callback);
      } else {
        gadgets.warn("cannot removeBrowserEvent: " + eventName);
      }
    },
  };
})();
gadgets["util"].getUrlParameters();
var tamings___ = tamings___ || [];
tamings___.push(function (imports) {
  caja___.whitelistFuncs([
    [gadgets.util, "escapeString"],
    [gadgets.util, "getFeatureParameters"],
    [gadgets.util, "getUrlParameters"],
    [gadgets.util, "hasFeature"],
    [gadgets.util, "registerOnLoadHandler"],
    [gadgets.util, "unescapeString"],
  ]);
});
shindig.Auth = function () {
  var authToken = null;
  var trusted = null;
  function addParamsToToken(urlParams) {
    var args = authToken.split("&");
    for (var i = 0; i < args.length; i++) {
      var nameAndValue = args[i].split("=");
      if (nameAndValue.length === 2) {
        var name = nameAndValue[0];
        var value = nameAndValue[1];
        if (value === "$") {
          value = encodeURIComponent(urlParams[name]);
          args[i] = name + "=" + value;
        }
      }
    }
    authToken = args.join("&");
  }
  function init(configuration) {
    var urlParams = gadgets.util.getUrlParameters();
    var config = configuration["shindig.auth"] || {};
    if (config.authToken) {
      authToken = config.authToken;
    } else if (urlParams.st) {
      authToken = urlParams.st;
    }
    if (authToken !== null) {
      addParamsToToken(urlParams);
    }
    if (config.trustedJson) {
      trusted = eval("(" + config.trustedJson + ")");
    }
  }
  gadgets.config.register("shindig.auth", null, init);
  return {
    getSecurityToken: function () {
      return authToken;
    },
    updateSecurityToken: function (newToken) {
      authToken = newToken;
    },
    getTrustedData: function () {
      return trusted;
    },
  };
};
shindig.auth = new shindig.Auth();
gadgets.io = (function () {
  var config = {};
  var oauthState;
  function makeXhr() {
    var x;
    if (
      typeof shindig != "undefined" &&
      shindig.xhrwrapper &&
      shindig.xhrwrapper.createXHR
    ) {
      return shindig.xhrwrapper.createXHR();
    } else if (typeof ActiveXObject != "undefined") {
      x = new ActiveXObject("Msxml2.XMLHTTP");
      if (!x) {
        x = new ActiveXObject("Microsoft.XMLHTTP");
      }
      return x;
    } else if (typeof XMLHttpRequest != "undefined" || window.XMLHttpRequest) {
      return new window.XMLHttpRequest();
    } else throw "no xhr available";
  }
  function hadError(xobj, callback) {
    if (xobj.readyState !== 4) {
      return true;
    }
    try {
      if (xobj.status !== 200) {
        var error = "" + xobj.status;
        if (xobj.responseText) {
          error = error + " " + xobj.responseText;
        }
        callback({
          errors: [error],
          rc: xobj.status,
          text: xobj.responseText,
        });
        return true;
      }
    } catch (e) {
      callback({
        errors: [e.number + " Error not specified"],
        rc: e.number,
        text: e.description,
      });
      return true;
    }
    return false;
  }
  function processNonProxiedResponse(url, callback, params, xobj) {
    if (hadError(xobj, callback)) {
      return;
    }
    var data = {
      body: xobj.responseText,
    };
    callback(transformResponseData(params, data));
  }
  var UNPARSEABLE_CRUFT = "throw 1; < don't be evil' >";
  function processResponse(url, callback, params, xobj) {
    if (hadError(xobj, callback)) {
      return;
    }
    var txt = xobj.responseText;
    var offset = txt.indexOf(UNPARSEABLE_CRUFT) + UNPARSEABLE_CRUFT.length;
    if (offset < UNPARSEABLE_CRUFT.length) return;
    txt = txt.substr(offset);
    var data = eval("(" + txt + ")");
    data = data[url];
    if (data.oauthState) {
      oauthState = data.oauthState;
    }
    if (data.st) {
      shindig.auth.updateSecurityToken(data.st);
    }
    callback(transformResponseData(params, data));
  }
  function transformResponseData(params, data) {
    var resp = {
      text: data.body,
      rc: data.rc || 200,
      headers: data.headers,
      oauthApprovalUrl: data.oauthApprovalUrl,
      oauthError: data.oauthError,
      oauthErrorText: data.oauthErrorText,
      errors: [],
    };
    if (resp.rc < 200 || resp.rc >= 400) {
      resp.errors = [resp.rc + " Error"];
    } else if (resp.text) {
      if (resp.rc >= 300 && resp.rc < 400) {
        params.CONTENT_TYPE = "TEXT";
      }
      switch (params.CONTENT_TYPE) {
        case "JSON":
        case "FEED":
          resp.data = gadgets.json.parse(resp.text);
          if (!resp.data) {
            resp.errors.push("500 Failed to parse JSON");
            resp.rc = 500;
            resp.data = null;
          }
          break;
        case "DOM":
          var dom;
          if (typeof ActiveXObject != "undefined") {
            dom = new ActiveXObject("Microsoft.XMLDOM");
            dom.async = false;
            dom.validateOnParse = false;
            dom.resolveExternals = false;
            if (!dom.loadXML(resp.text)) {
              resp.errors.push("500 Failed to parse XML");
              resp.rc = 500;
            } else {
              resp.data = dom;
            }
          } else {
            var parser = new DOMParser();
            dom = parser.parseFromString(resp.text, "text/xml");
            if ("parsererror" === dom.documentElement.nodeName) {
              resp.errors.push("500 Failed to parse XML");
              resp.rc = 500;
            } else {
              resp.data = dom;
            }
          }
          break;
        default:
          resp.data = resp.text;
          break;
      }
    }
    return resp;
  }
  function makeXhrRequest(
    realUrl,
    proxyUrl,
    callback,
    paramData,
    method,
    params,
    processResponseFunction,
    opt_contentType,
  ) {
    var xhr = makeXhr();
    if (proxyUrl.indexOf("//") == 0) {
      proxyUrl = document.location.protocol + proxyUrl;
    }
    xhr.open(method, proxyUrl, true);
    if (callback) {
      xhr.onreadystatechange = gadgets.util.makeClosure(
        null,
        processResponseFunction,
        realUrl,
        callback,
        params,
        xhr,
      );
    }
    if (paramData !== null) {
      xhr.setRequestHeader(
        "Content-Type",
        opt_contentType || "application/x-www-form-urlencoded",
      );
      xhr.send(paramData);
    } else {
      xhr.send(null);
    }
  }
  function respondWithPreload(postData, params, callback) {
    if (gadgets.io.preloaded_ && postData.httpMethod === "GET") {
      for (var i = 0; i < gadgets.io.preloaded_.length; i++) {
        var preload = gadgets.io.preloaded_[i];
        if (preload && preload.id === postData.url) {
          delete gadgets.io.preloaded_[i];
          if (preload.rc !== 200) {
            callback({
              rc: preload.rc,
              errors: [preload.rc + " Error"],
            });
          } else {
            if (preload.oauthState) {
              oauthState = preload.oauthState;
            }
            var resp = {
              body: preload.body,
              rc: preload.rc,
              headers: preload.headers,
              oauthApprovalUrl: preload.oauthApprovalUrl,
              oauthError: preload.oauthError,
              oauthErrorText: preload.oauthErrorText,
              errors: [],
            };
            callback(transformResponseData(params, resp));
          }
          return true;
        }
      }
    }
    return false;
  }
  function init(configuration) {
    config = configuration["core.io"] || {};
  }
  var requiredConfig = {
    proxyUrl: new gadgets.config.RegExValidator(/.*%(raw)?url%.*/),
    jsonProxyUrl: gadgets.config.NonEmptyStringValidator,
  };
  gadgets.config.register("core.io", requiredConfig, init);
  return {
    makeRequest: function (url, callback, opt_params) {
      var params = opt_params || {};
      var httpMethod = params.METHOD || "GET";
      var refreshInterval = params.REFRESH_INTERVAL;
      var auth, st;
      if (params.AUTHORIZATION && params.AUTHORIZATION !== "NONE") {
        auth = params.AUTHORIZATION.toLowerCase();
        st = shindig.auth.getSecurityToken();
      } else {
        if (httpMethod === "GET" && refreshInterval === undefined) {
          refreshInterval = 3600;
        }
      }
      var signOwner = true;
      if (typeof params.OWNER_SIGNED !== "undefined") {
        signOwner = params.OWNER_SIGNED;
      }
      var signViewer = true;
      if (typeof params.VIEWER_SIGNED !== "undefined") {
        signViewer = params.VIEWER_SIGNED;
      }
      var headers = params.HEADERS || {};
      if (httpMethod === "POST" && !headers["Content-Type"]) {
        headers["Content-Type"] = "application/x-www-form-urlencoded";
      }
      var urlParams = gadgets.util.getUrlParameters();
      var paramData = {
        url: url,
        httpMethod: httpMethod,
        headers: gadgets.io.encodeValues(headers, false),
        postData: params.POST_DATA || "",
        authz: auth || "",
        st: st || "",
        contentType: params.CONTENT_TYPE || "TEXT",
        numEntries: params.NUM_ENTRIES || "3",
        getSummaries: !!params.GET_SUMMARIES,
        signOwner: signOwner,
        signViewer: signViewer,
        gadget: urlParams.url,
        container: urlParams.container || urlParams.synd || "default",
        bypassSpecCache: gadgets.util.getUrlParameters().nocache || "",
        getFullHeaders: !!params.GET_FULL_HEADERS,
      };
      if (auth === "oauth" || auth === "signed") {
        if (gadgets.io.oauthReceivedCallbackUrl_) {
          paramData.OAUTH_RECEIVED_CALLBACK =
            gadgets.io.oauthReceivedCallbackUrl_;
          gadgets.io.oauthReceivedCallbackUrl_ = null;
        }
        paramData.oauthState = oauthState || "";
        for (var opt in params) {
          if (params.hasOwnProperty(opt)) {
            if (opt.indexOf("OAUTH_") === 0) {
              paramData[opt] = params[opt];
            }
          }
        }
      }
      var proxyUrl = config.jsonProxyUrl.replace(
        "%host%",
        document.location.host,
      );
      if (!respondWithPreload(paramData, params, callback, processResponse)) {
        if (httpMethod === "GET" && refreshInterval > 0) {
          var extraparams =
            "?refresh=" +
            refreshInterval +
            "&" +
            gadgets.io.encodeValues(paramData);
          makeXhrRequest(
            url,
            proxyUrl + extraparams,
            callback,
            null,
            "GET",
            params,
            processResponse,
          );
        } else {
          makeXhrRequest(
            url,
            proxyUrl,
            callback,
            gadgets.io.encodeValues(paramData),
            "POST",
            params,
            processResponse,
          );
        }
      }
    },
    makeNonProxiedRequest: function (
      relativeUrl,
      callback,
      opt_params,
      opt_contentType,
    ) {
      var params = opt_params || {};
      makeXhrRequest(
        relativeUrl,
        relativeUrl,
        callback,
        params.POST_DATA,
        params.METHOD,
        params,
        processNonProxiedResponse,
        opt_contentType,
      );
    },
    clearOAuthState: function () {
      oauthState = undefined;
    },
    encodeValues: function (fields, opt_noEscaping) {
      var escape = !opt_noEscaping;
      var buf = [];
      var first = false;
      for (var i in fields) {
        if (fields.hasOwnProperty(i) && !/___$/.test(i)) {
          if (!first) {
            first = true;
          } else {
            buf.push("&");
          }
          buf.push(escape ? encodeURIComponent(i) : i);
          buf.push("=");
          buf.push(escape ? encodeURIComponent(fields[i]) : fields[i]);
        }
      }
      return buf.join("");
    },
    getProxyUrl: function (url, opt_params) {
      var params = opt_params || {};
      var refresh = params.REFRESH_INTERVAL;
      if (refresh === undefined) {
        refresh = "3600";
      }
      var urlParams = gadgets.util.getUrlParameters();
      var rewriteMimeParam = params.rewriteMime
        ? "&rewriteMime=" + encodeURIComponent(params.rewriteMime)
        : "";
      var ret = config.proxyUrl
        .replace("%url%", encodeURIComponent(url))
        .replace("%host%", document.location.host)
        .replace("%rawurl%", url)
        .replace("%refresh%", encodeURIComponent(refresh))
        .replace("%gadget%", encodeURIComponent(urlParams.url))
        .replace(
          "%container%",
          encodeURIComponent(
            urlParams.container || urlParams.synd || "default",
          ),
        )
        .replace("%rewriteMime%", rewriteMimeParam);
      if (ret.indexOf("//") == 0) {
        ret = window.location.protocol + ret;
      }
      return ret;
    },
  };
})();
gadgets.io.RequestParameters = gadgets.util.makeEnum([
  "METHOD",
  "CONTENT_TYPE",
  "POST_DATA",
  "HEADERS",
  "AUTHORIZATION",
  "NUM_ENTRIES",
  "GET_SUMMARIES",
  "GET_FULL_HEADERS",
  "REFRESH_INTERVAL",
  "OAUTH_SERVICE_NAME",
  "OAUTH_USE_TOKEN",
  "OAUTH_TOKEN_NAME",
  "OAUTH_REQUEST_TOKEN",
  "OAUTH_REQUEST_TOKEN_SECRET",
  "OAUTH_RECEIVED_CALLBACK",
]);
gadgets.io.MethodType = gadgets.util.makeEnum([
  "GET",
  "POST",
  "PUT",
  "DELETE",
  "HEAD",
]);
gadgets.io.ContentType = gadgets.util.makeEnum(["TEXT", "DOM", "JSON", "FEED"]);
gadgets.io.AuthorizationType = gadgets.util.makeEnum([
  "NONE",
  "SIGNED",
  "OAUTH",
]);
var tamings___ = tamings___ || [];
tamings___.push(function (imports) {
  caja___.whitelistFuncs([
    [gadgets.io, "encodeValues"],
    [gadgets.io, "getProxyUrl"],
    [gadgets.io, "makeRequest"],
  ]);
});
gadgets.rpctx = gadgets.rpctx || {};
if (!gadgets.rpctx.wpm) {
  gadgets.rpctx.wpm = (function () {
    var process, ready;
    var postMessage;
    var pmSync = false;
    var pmEventDomain = false;
    var isForceSecure = false;
    function testPostMessage() {
      var hit = false;
      function receiveMsg(event) {
        if (event.data == "postmessage.test") {
          hit = true;
          if (typeof event.origin === "undefined") {
            pmEventDomain = true;
          }
        }
      }
      gadgets.util.attachBrowserEvent(window, "message", receiveMsg, false);
      window.postMessage("postmessage.test", "*");
      if (hit) {
        pmSync = true;
      }
      gadgets.util.removeBrowserEvent(window, "message", receiveMsg, false);
    }
    function onmessage(packet) {
      var rpc = gadgets.json.parse(packet.data);
      if (isForceSecure) {
        if (!rpc || !rpc.f) {
          return;
        }
        var origRelay =
          gadgets.rpc.getRelayUrl(rpc.f) ||
          gadgets.util.getUrlParameters()["parent"];
        var origin = gadgets.rpc.getOrigin(origRelay);
        if (
          !pmEventDomain
            ? packet.origin !== origin
            : packet.domain !== /^.+:\/\/([^:]+).*/.exec(origin)[1]
        ) {
          return;
        }
      }
      process(rpc);
    }
    return {
      getCode: function () {
        return "wpm";
      },
      isParentVerifiable: function () {
        return true;
      },
      init: function (processFn, readyFn) {
        process = processFn;
        ready = readyFn;
        testPostMessage();
        if (!pmSync) {
          postMessage = function (win, msg, origin) {
            win.postMessage(msg, origin);
          };
        } else {
          postMessage = function (win, msg, origin) {
            window.setTimeout(function () {
              win.postMessage(msg, origin);
            }, 0);
          };
        }
        gadgets.util.attachBrowserEvent(window, "message", onmessage, false);
        ready("..", true);
        return true;
      },
      setup: function (receiverId, token, forceSecure) {
        isForceSecure = forceSecure;
        if (receiverId === "..") {
          if (isForceSecure) {
            gadgets.rpc._createRelayIframe(token);
          } else {
            gadgets.rpc.call(receiverId, gadgets.rpc.ACK);
          }
        }
        return true;
      },
      call: function (targetId, from, rpc) {
        var targetWin = gadgets.rpc._getTargetWin(targetId);
        var origRelay =
          gadgets.rpc.getRelayUrl(targetId) ||
          gadgets.util.getUrlParameters()["parent"];
        var origin = gadgets.rpc.getOrigin(origRelay);
        if (origin) {
          postMessage(targetWin, gadgets.json.stringify(rpc), origin);
        } else {
          gadgets.error(
            "No relay set (used as window.postMessage targetOrigin)" +
              ", cannot send cross-domain message",
          );
        }
        return true;
      },
      relayOnload: function (receiverId, data) {
        ready(receiverId, true);
      },
    };
  })();
}
gadgets.rpctx = gadgets.rpctx || {};
if (!gadgets.rpctx.frameElement) {
  gadgets.rpctx.frameElement = (function () {
    var FE_G2C_CHANNEL = "__g2c_rpc";
    var FE_C2G_CHANNEL = "__c2g_rpc";
    var process;
    var ready;
    function callFrameElement(targetId, from, rpc) {
      try {
        if (from !== "..") {
          var fe = window.frameElement;
          if (typeof fe[FE_G2C_CHANNEL] === "function") {
            if (typeof fe[FE_G2C_CHANNEL][FE_C2G_CHANNEL] !== "function") {
              fe[FE_G2C_CHANNEL][FE_C2G_CHANNEL] = function (args) {
                process(gadgets.json.parse(args));
              };
            }
            fe[FE_G2C_CHANNEL](gadgets.json.stringify(rpc));
            return true;
          }
        } else {
          var frame = document.getElementById(targetId);
          if (
            typeof frame[FE_G2C_CHANNEL] === "function" &&
            typeof frame[FE_G2C_CHANNEL][FE_C2G_CHANNEL] === "function"
          ) {
            frame[FE_G2C_CHANNEL][FE_C2G_CHANNEL](gadgets.json.stringify(rpc));
            return true;
          }
        }
      } catch (e) {}
      return false;
    }
    return {
      getCode: function () {
        return "fe";
      },
      isParentVerifiable: function () {
        return false;
      },
      init: function (processFn, readyFn) {
        process = processFn;
        ready = readyFn;
        return true;
      },
      setup: function (receiverId, token) {
        if (receiverId !== "..") {
          try {
            var frame = document.getElementById(receiverId);
            frame[FE_G2C_CHANNEL] = function (args) {
              process(gadgets.json.parse(args));
            };
          } catch (e) {
            return false;
          }
        }
        if (receiverId === "..") {
          ready("..", true);
          var ackFn = function () {
            window.setTimeout(function () {
              gadgets.rpc.call(receiverId, gadgets.rpc.ACK);
            }, 500);
          };
          gadgets.util.registerOnLoadHandler(ackFn);
        }
        return true;
      },
      call: function (targetId, from, rpc) {
        return callFrameElement(targetId, from, rpc);
      },
    };
  })();
}
gadgets.rpctx = gadgets.rpctx || {};
if (!gadgets.rpctx.nix) {
  gadgets.rpctx.nix = (function () {
    var NIX_WRAPPER = "GRPC____NIXVBS_wrapper";
    var NIX_GET_WRAPPER = "GRPC____NIXVBS_get_wrapper";
    var NIX_HANDLE_MESSAGE = "GRPC____NIXVBS_handle_message";
    var NIX_CREATE_CHANNEL = "GRPC____NIXVBS_create_channel";
    var MAX_NIX_SEARCHES = 10;
    var NIX_SEARCH_PERIOD = 500;
    var nix_channels = {};
    var isForceSecure = {};
    var ready;
    var numHandlerSearches = 0;
    function conductHandlerSearch() {
      var handler = nix_channels[".."];
      if (handler) {
        return;
      }
      if (++numHandlerSearches > MAX_NIX_SEARCHES) {
        gadgets.warn("Nix transport setup failed, falling back...");
        ready("..", false);
        return;
      }
      if (!handler && window.opener && "GetAuthToken" in window.opener) {
        handler = window.opener;
        if (handler.GetAuthToken() == gadgets.rpc.getAuthToken("..")) {
          var token = gadgets.rpc.getAuthToken("..");
          handler.CreateChannel(window[NIX_GET_WRAPPER]("..", token), token);
          nix_channels[".."] = handler;
          window.opener = null;
          ready("..", true);
          return;
        }
      }
      window.setTimeout(function () {
        conductHandlerSearch();
      }, NIX_SEARCH_PERIOD);
    }
    function getLocationNoHash() {
      var loc = window.location.href;
      var idx = loc.indexOf("#");
      if (idx == -1) {
        return loc;
      }
      return loc.substring(0, idx);
    }
    function setupSecureRelayToParent(rpctoken) {
      var childToken = (0x7fffffff * Math.random()) | 0;
      var data = [getLocationNoHash(), childToken];
      gadgets.rpc._createRelayIframe(rpctoken, data);
      var hash = window.location.href.split("#")[1] || "";
      function relayTimer() {
        var newHash = window.location.href.split("#")[1] || "";
        if (newHash !== hash) {
          clearInterval(relayTimerId);
          var params = gadgets.util.getUrlParameters(window.location.href);
          if (params.childtoken == childToken) {
            conductHandlerSearch();
            return;
          }
          ready("..", false);
        }
      }
      var relayTimerId = setInterval(relayTimer, 100);
    }
    return {
      getCode: function () {
        return "nix";
      },
      isParentVerifiable: function (opt_receiverId) {
        if (opt_receiverId) {
          return isForceSecure[opt_receiverId];
        }
        return false;
      },
      init: function (processFn, readyFn) {
        ready = readyFn;
        if (typeof window[NIX_GET_WRAPPER] !== "unknown") {
          window[NIX_HANDLE_MESSAGE] = function (data) {
            window.setTimeout(function () {
              processFn(gadgets.json.parse(data));
            }, 0);
          };
          window[NIX_CREATE_CHANNEL] = function (name, channel, token) {
            if (gadgets.rpc.getAuthToken(name) === token) {
              nix_channels[name] = channel;
              ready(name, true);
            }
          };
          var vbscript =
            "Class " +
            NIX_WRAPPER +
            "\n " +
            "Private m_Intended\n" +
            "Private m_Auth\n" +
            "Public Sub SetIntendedName(name)\n " +
            "If isEmpty(m_Intended) Then\n" +
            "m_Intended = name\n" +
            "End If\n" +
            "End Sub\n" +
            "Public Sub SetAuth(auth)\n " +
            "If isEmpty(m_Auth) Then\n" +
            "m_Auth = auth\n" +
            "End If\n" +
            "End Sub\n" +
            "Public Sub SendMessage(data)\n " +
            NIX_HANDLE_MESSAGE +
            "(data)\n" +
            "End Sub\n" +
            "Public Function GetAuthToken()\n " +
            "GetAuthToken = m_Auth\n" +
            "End Function\n" +
            "Public Sub CreateChannel(channel, auth)\n " +
            "Call " +
            NIX_CREATE_CHANNEL +
            "(m_Intended, channel, auth)\n" +
            "End Sub\n" +
            "End Class\n" +
            "Function " +
            NIX_GET_WRAPPER +
            "(name, auth)\n" +
            "Dim wrap\n" +
            "Set wrap = New " +
            NIX_WRAPPER +
            "\n" +
            "wrap.SetIntendedName name\n" +
            "wrap.SetAuth auth\n" +
            "Set " +
            NIX_GET_WRAPPER +
            " = wrap\n" +
            "End Function";
          try {
            window.execScript(vbscript, "vbscript");
          } catch (e) {
            return false;
          }
        }
        return true;
      },
      setup: function (receiverId, token, forcesecure) {
        isForceSecure[receiverId] = !!forcesecure;
        if (receiverId === "..") {
          if (forcesecure) {
            setupSecureRelayToParent(token);
          } else {
            conductHandlerSearch();
          }
          return true;
        }
        try {
          var frame = document.getElementById(receiverId);
          var wrapper = window[NIX_GET_WRAPPER](receiverId, token);
          frame.contentWindow.opener = wrapper;
        } catch (e) {
          return false;
        }
        return true;
      },
      call: function (targetId, from, rpc) {
        try {
          if (nix_channels[targetId]) {
            nix_channels[targetId].SendMessage(gadgets.json.stringify(rpc));
          }
        } catch (e) {
          return false;
        }
        return true;
      },
      relayOnload: function (receiverId, data) {
        var src = data[0] + "#childtoken=" + data[1];
        var childIframe = document.getElementById(receiverId);
        childIframe.src = src;
      },
    };
  })();
}
gadgets.rpctx = gadgets.rpctx || {};
if (!gadgets.rpctx.rmr) {
  gadgets.rpctx.rmr = (function () {
    var RMR_SEARCH_TIMEOUT = 500;
    var RMR_MAX_POLLS = 10;
    var rmr_channels = {};
    var process;
    var ready;
    function appendRmrFrame(channelFrame, relayUri, data, opt_frameId) {
      var appendFn = function () {
        document.body.appendChild(channelFrame);
        channelFrame.src = "about:blank";
        if (opt_frameId) {
          channelFrame.onload = function () {
            processRmrData(opt_frameId);
          };
        }
        channelFrame.src = relayUri + "#" + data;
      };
      if (document.body) {
        appendFn();
      } else {
        gadgets.util.registerOnLoadHandler(function () {
          appendFn();
        });
      }
    }
    function setupRmr(frameId) {
      if (typeof rmr_channels[frameId] === "object") {
        return;
      }
      var channelFrame = document.createElement("iframe");
      var frameStyle = channelFrame.style;
      frameStyle.position = "absolute";
      frameStyle.top = "0px";
      frameStyle.border = "0";
      frameStyle.opacity = "0";
      frameStyle.width = "10px";
      frameStyle.height = "1px";
      channelFrame.id = "rmrtransport-" + frameId;
      channelFrame.name = channelFrame.id;
      var relayUri = gadgets.rpc.getRelayUrl(frameId);
      if (!relayUri) {
        relayUri =
          gadgets.rpc.getOrigin(gadgets.util.getUrlParameters()["parent"]) +
          "/robots.txt";
      }
      rmr_channels[frameId] = {
        frame: channelFrame,
        receiveWindow: null,
        relayUri: relayUri,
        searchCounter: 0,
        width: 10,
        waiting: true,
        queue: [],
        sendId: 0,
        recvId: 0,
      };
      if (frameId !== "..") {
        appendRmrFrame(channelFrame, relayUri, getRmrData(frameId));
      }
      conductRmrSearch(frameId);
    }
    function conductRmrSearch(frameId) {
      var channelWindow = null;
      rmr_channels[frameId].searchCounter++;
      try {
        var targetWin = gadgets.rpc._getTargetWin(frameId);
        if (frameId === "..") {
          channelWindow =
            targetWin.frames["rmrtransport-" + gadgets.rpc.RPC_ID];
        } else {
          channelWindow = targetWin.frames["rmrtransport-.."];
        }
      } catch (e) {}
      var status = false;
      if (channelWindow) {
        status = registerRmrChannel(frameId, channelWindow);
      }
      if (!status) {
        if (rmr_channels[frameId].searchCounter > RMR_MAX_POLLS) {
          return;
        }
        window.setTimeout(function () {
          conductRmrSearch(frameId);
        }, RMR_SEARCH_TIMEOUT);
      }
    }
    function callRmr(targetId, serviceName, from, rpc) {
      var handler = null;
      if (from !== "..") {
        handler = rmr_channels[".."];
      } else {
        handler = rmr_channels[targetId];
      }
      if (handler) {
        if (serviceName !== gadgets.rpc.ACK) {
          handler.queue.push(rpc);
        }
        if (
          handler.waiting ||
          (handler.queue.length === 0 &&
            !(serviceName === gadgets.rpc.ACK && rpc && rpc.ackAlone === true))
        ) {
          return true;
        }
        if (handler.queue.length > 0) {
          handler.waiting = true;
        }
        var url = handler.relayUri + "#" + getRmrData(targetId);
        try {
          handler.frame.contentWindow.location = url;
          var newWidth = handler.width == 10 ? 20 : 10;
          handler.frame.style.width = newWidth + "px";
          handler.width = newWidth;
        } catch (e) {
          return false;
        }
      }
      return true;
    }
    function getRmrData(toFrameId) {
      var channel = rmr_channels[toFrameId];
      var rmrData = {
        id: channel.sendId,
      };
      if (channel) {
        rmrData.d = Array.prototype.slice.call(channel.queue, 0);
        rmrData.d.push({
          s: gadgets.rpc.ACK,
          id: channel.recvId,
        });
      }
      return gadgets.json.stringify(rmrData);
    }
    function processRmrData(fromFrameId) {
      var channel = rmr_channels[fromFrameId];
      var data = channel.receiveWindow.location.hash.substring(1);
      var rpcObj = gadgets.json.parse(decodeURIComponent(data)) || {};
      var rpcArray = rpcObj.d || [];
      var nonAckReceived = false;
      var noLongerWaiting = false;
      var numBypassed = 0;
      var numToBypass = channel.recvId - rpcObj.id;
      for (var i = 0; i < rpcArray.length; ++i) {
        var rpc = rpcArray[i];
        if (rpc.s === gadgets.rpc.ACK) {
          ready(fromFrameId, true);
          if (channel.waiting) {
            noLongerWaiting = true;
          }
          channel.waiting = false;
          var newlyAcked = Math.max(0, rpc.id - channel.sendId);
          channel.queue.splice(0, newlyAcked);
          channel.sendId = Math.max(channel.sendId, rpc.id || 0);
          continue;
        }
        nonAckReceived = true;
        if (++numBypassed <= numToBypass) {
          continue;
        }
        ++channel.recvId;
        process(rpc);
      }
      if (nonAckReceived || (noLongerWaiting && channel.queue.length > 0)) {
        var from = fromFrameId === ".." ? gadgets.rpc.RPC_ID : "..";
        callRmr(fromFrameId, gadgets.rpc.ACK, from, {
          ackAlone: nonAckReceived,
        });
      }
    }
    function registerRmrChannel(frameId, channelWindow) {
      var channel = rmr_channels[frameId];
      try {
        var canAccess = false;
        canAccess = "document" in channelWindow;
        if (!canAccess) {
          return false;
        }
        canAccess = typeof channelWindow["document"] == "object";
        if (!canAccess) {
          return false;
        }
        var loc = channelWindow.location.href;
        if (loc === "about:blank") {
          return false;
        }
      } catch (ex) {
        return false;
      }
      channel.receiveWindow = channelWindow;
      function onresize() {
        processRmrData(frameId);
      }
      if (typeof channelWindow.attachEvent === "undefined") {
        channelWindow.onresize = onresize;
      } else {
        channelWindow.attachEvent("onresize", onresize);
      }
      if (frameId === "..") {
        appendRmrFrame(
          channel.frame,
          channel.relayUri,
          getRmrData(frameId),
          frameId,
        );
      } else {
        processRmrData(frameId);
      }
      return true;
    }
    return {
      getCode: function () {
        return "rmr";
      },
      isParentVerifiable: function () {
        return true;
      },
      init: function (processFn, readyFn) {
        process = processFn;
        ready = readyFn;
        return true;
      },
      setup: function (receiverId, token) {
        try {
          setupRmr(receiverId);
        } catch (e) {
          gadgets.warn("Caught exception setting up RMR: " + e);
          return false;
        }
        return true;
      },
      call: function (targetId, from, rpc) {
        return callRmr(targetId, rpc.s, from, rpc);
      },
    };
  })();
}
gadgets.rpctx = gadgets.rpctx || {};
if (!gadgets.rpctx.ifpc) {
  gadgets.rpctx.ifpc = (function () {
    var iframePool = [];
    var callId = 0;
    var ready;
    function encodeLegacyData(args) {
      var argsEscaped = [];
      for (var i = 0, j = args.length; i < j; ++i) {
        argsEscaped.push(encodeURIComponent(gadgets.json.stringify(args[i])));
      }
      return argsEscaped.join("&");
    }
    function emitInvisibleIframe(src) {
      var iframe;
      for (var i = iframePool.length - 1; i >= 0; --i) {
        var ifr = iframePool[i];
        try {
          if (ifr && (ifr.recyclable || ifr.readyState === "complete")) {
            ifr.parentNode.removeChild(ifr);
            if (window.ActiveXObject) {
              iframePool[i] = ifr = null;
              iframePool.splice(i, 1);
            } else {
              ifr.recyclable = false;
              iframe = ifr;
              break;
            }
          }
        } catch (e) {}
      }
      if (!iframe) {
        iframe = document.createElement("iframe");
        iframe.style.border = iframe.style.width = iframe.style.height = "0px";
        iframe.style.visibility = "hidden";
        iframe.style.position = "absolute";
        iframe.onload = function () {
          this.recyclable = true;
        };
        iframePool.push(iframe);
      }
      iframe.src = src;
      window.setTimeout(function () {
        document.body.appendChild(iframe);
      }, 0);
    }
    return {
      getCode: function () {
        return "ifpc";
      },
      isParentVerifiable: function () {
        return true;
      },
      init: function (processFn, readyFn) {
        ready = readyFn;
        ready("..", true);
        return true;
      },
      setup: function (receiverId, token) {
        ready(receiverId, true);
        return true;
      },
      call: function (targetId, from, rpc) {
        var relay = gadgets.rpc.getRelayUrl(targetId);
        ++callId;
        if (!relay) {
          gadgets.warn("No relay file assigned for IFPC");
          return false;
        }
        var src = null;
        if (rpc.l) {
          var callArgs = rpc.a;
          src = [
            relay,
            "#",
            encodeLegacyData([
              from,
              callId,
              1,
              0,
              encodeLegacyData([from, rpc.s, "", "", from].concat(callArgs)),
            ]),
          ].join("");
        } else {
          src = [
            relay,
            "#",
            targetId,
            "&",
            from,
            "@",
            callId,
            "&1&0&",
            encodeURIComponent(gadgets.json.stringify(rpc)),
          ].join("");
        }
        emitInvisibleIframe(src);
        return true;
      },
    };
  })();
}
if (!gadgets.rpc) {
  gadgets.rpc = (function () {
    var CALLBACK_NAME = "__cb";
    var DEFAULT_NAME = "";
    var ACK = "__ack";
    var SETUP_FRAME_TIMEOUT = 500;
    var SETUP_FRAME_MAX_TRIES = 10;
    var services = {};
    var relayUrl = {};
    var useLegacyProtocol = {};
    var authToken = {};
    var callId = 0;
    var callbacks = {};
    var setup = {};
    var sameDomain = {};
    var params = {};
    var receiverTx = {};
    var earlyRpcQueue = {};
    var isChild = window.top !== window.self;
    var rpcId = window.name;
    var securityCallback = function () {};
    var LOAD_TIMEOUT = 0;
    var FRAME_PHISH = 1;
    var FORGED_MSG = 2;
    var fallbackTransport = (function () {
      function logFn(name) {
        return function () {
          gadgets.log(
            "gadgets.rpc." +
              name +
              "(" +
              gadgets.json.stringify(Array.prototype.slice.call(arguments)) +
              "): call ignored. [caller: " +
              document.location +
              ", isChild: " +
              isChild +
              "]",
          );
        };
      }
      return {
        getCode: function () {
          return "noop";
        },
        isParentVerifiable: function () {
          return true;
        },
        init: logFn("init"),
        setup: logFn("setup"),
        call: logFn("call"),
      };
    })();
    if (gadgets.util) {
      params = gadgets.util.getUrlParameters();
    }
    function getTransport() {
      return typeof window.postMessage === "function"
        ? gadgets.rpctx.wpm
        : typeof window.postMessage === "object"
          ? gadgets.rpctx.wpm
          : window.ActiveXObject
            ? gadgets.rpctx.nix
            : navigator.userAgent.indexOf("WebKit") > 0
              ? gadgets.rpctx.rmr
              : navigator.product === "Gecko"
                ? gadgets.rpctx.frameElement
                : gadgets.rpctx.ifpc;
    }
    function transportReady(receiverId, readySuccess) {
      var tx = transport;
      if (!readySuccess) {
        tx = fallbackTransport;
      }
      receiverTx[receiverId] = tx;
      var earlyQueue = earlyRpcQueue[receiverId] || [];
      for (var i = 0; i < earlyQueue.length; ++i) {
        var rpc = earlyQueue[i];
        rpc.t = getAuthToken(receiverId);
        tx.call(receiverId, rpc.f, rpc);
      }
      earlyRpcQueue[receiverId] = [];
    }
    var mainPageUnloading = false,
      hookedUnload = false;
    function hookMainPageUnload() {
      if (hookedUnload) {
        return;
      }
      function onunload() {
        mainPageUnloading = true;
      }
      gadgets.util.attachBrowserEvent(window, "unload", onunload, false);
      hookedUnload = true;
    }
    function relayOnload(targetId, sourceId, token, data, relayWindow) {
      if (!authToken[sourceId] || authToken[sourceId] !== token) {
        gadgets.error(
          "Invalid auth token. " + authToken[sourceId] + " vs " + token,
        );
        securityCallback(sourceId, FORGED_MSG);
      }
      relayWindow.onunload = function () {
        if (setup[sourceId] && !mainPageUnloading) {
          securityCallback(sourceId, FRAME_PHISH);
          gadgets.rpc.removeReceiver(sourceId);
        }
      };
      hookMainPageUnload();
      data = gadgets.json.parse(decodeURIComponent(data));
      transport.relayOnload(sourceId, data);
    }
    function process(rpc) {
      if (
        rpc &&
        typeof rpc.s === "string" &&
        typeof rpc.f === "string" &&
        rpc.a instanceof Array
      ) {
        if (authToken[rpc.f]) {
          if (authToken[rpc.f] !== rpc.t) {
            gadgets.error(
              "Invalid auth token. " + authToken[rpc.f] + " vs " + rpc.t,
            );
            securityCallback(rpc.f, FORGED_MSG);
          }
        }
        if (rpc.s === ACK) {
          window.setTimeout(function () {
            transportReady(rpc.f, true);
          }, 0);
          return;
        }
        if (rpc.c) {
          rpc.callback = function (result) {
            gadgets.rpc.call(rpc.f, CALLBACK_NAME, null, rpc.c, result);
          };
        }
        var result = (services[rpc.s] || services[DEFAULT_NAME]).apply(
          rpc,
          rpc.a,
        );
        if (rpc.c && typeof result !== "undefined") {
          gadgets.rpc.call(rpc.f, CALLBACK_NAME, null, rpc.c, result);
        }
      }
    }
    function getOrigin(url) {
      if (!url) {
        return "";
      }
      url = url.toLowerCase();
      if (url.indexOf("//") == 0) {
        url = window.location.protocol + url;
      }
      if (url.indexOf("://") == -1) {
        url = window.location.protocol + "//" + url;
      }
      var host = url.substring(url.indexOf("://") + 3);
      var slashPos = host.indexOf("/");
      if (slashPos != -1) {
        host = host.substring(0, slashPos);
      }
      var protocol = url.substring(0, url.indexOf("://"));
      var portStr = "";
      var portPos = host.indexOf(":");
      if (portPos != -1) {
        var port = host.substring(portPos + 1);
        host = host.substring(0, portPos);
        if (
          (protocol === "http" && port !== "80") ||
          (protocol === "https" && port !== "443")
        ) {
          portStr = ":" + port;
        }
      }
      return protocol + "://" + host + portStr;
    }
    function getTargetWin(id) {
      if (typeof id === "undefined" || id === "..") {
        return window.parent;
      }
      id = String(id);
      var target = window.frames[id];
      if (target) {
        return target;
      }
      target = document.getElementById(id);
      if (target && target.contentWindow) {
        return target.contentWindow;
      }
      return null;
    }
    var transport = getTransport();
    services[DEFAULT_NAME] = function () {
      gadgets.warn("Unknown RPC service: " + this.s);
    };
    services[CALLBACK_NAME] = function (callbackId, result) {
      var callback = callbacks[callbackId];
      if (callback) {
        delete callbacks[callbackId];
        callback(result);
      }
    };
    function setupFrame(frameId, token, forcesecure) {
      if (setup[frameId] === true) {
        return;
      }
      if (typeof setup[frameId] === "undefined") {
        setup[frameId] = 0;
      }
      var tgtFrame = document.getElementById(frameId);
      if (frameId === ".." || tgtFrame != null) {
        if (transport.setup(frameId, token, forcesecure) === true) {
          setup[frameId] = true;
          return;
        }
      }
      if (setup[frameId] !== true && setup[frameId]++ < SETUP_FRAME_MAX_TRIES) {
        window.setTimeout(function () {
          setupFrame(frameId, token, forcesecure);
        }, SETUP_FRAME_TIMEOUT);
      } else {
        receiverTx[frameId] = fallbackTransport;
        setup[frameId] = true;
      }
    }
    function callSameDomain(target, rpc) {
      if (typeof sameDomain[target] === "undefined") {
        sameDomain[target] = false;
        var targetRelay = gadgets.rpc.getRelayUrl(target);
        if (getOrigin(targetRelay) !== getOrigin(window.location.href)) {
          return false;
        }
        var targetEl = getTargetWin(target);
        try {
          sameDomain[target] = targetEl.gadgets.rpc.receiveSameDomain;
        } catch (e) {
          gadgets.error("Same domain call failed: parent= incorrectly set.");
        }
      }
      if (typeof sameDomain[target] === "function") {
        sameDomain[target](rpc);
        return true;
      }
      return false;
    }
    function setRelayUrl(targetId, url, opt_useLegacy) {
      if (!/http(s)?:\/\/.+/.test(url)) {
        if (url.indexOf("//") == 0) {
          url = window.location.protocol + url;
        } else if (url.charAt(0) == "/") {
          url = window.location.protocol + "//" + window.location.host + url;
        } else if (url.indexOf("://") == -1) {
          url = window.location.protocol + "//" + url;
        }
      }
      relayUrl[targetId] = url;
      useLegacyProtocol[targetId] = !!opt_useLegacy;
    }
    function getAuthToken(targetId) {
      return authToken[targetId];
    }
    function setAuthToken(targetId, token, forcesecure) {
      token = token || "";
      authToken[targetId] = String(token);
      setupFrame(targetId, token, forcesecure);
    }
    function setupContainerGadgetContext(rpctoken, opt_forcesecure) {
      function init(config) {
        var configRpc = config ? config.rpc : {};
        var parentRelayUrl = configRpc.parentRelayUrl;
        if (
          parentRelayUrl.substring(0, 7) !== "http://" &&
          parentRelayUrl.substring(0, 8) !== "https://" &&
          parentRelayUrl.substring(0, 2) !== "//"
        ) {
          if (typeof params.parent === "string" && params.parent !== "") {
            if (parentRelayUrl.substring(0, 1) !== "/") {
              var lastSlash = params.parent.lastIndexOf("/");
              parentRelayUrl =
                params.parent.substring(0, lastSlash + 1) + parentRelayUrl;
            } else {
              parentRelayUrl = getOrigin(params.parent) + parentRelayUrl;
            }
          }
        }
        var useLegacy = !!configRpc.useLegacyProtocol;
        setRelayUrl("..", parentRelayUrl, useLegacy);
        if (useLegacy) {
          transport = gadgets.rpctx.ifpc;
          transport.init(process, transportReady);
        }
        var forceSecure = opt_forcesecure || params.forcesecure || false;
        setAuthToken("..", rpctoken, forceSecure);
      }
      var requiredConfig = {
        parentRelayUrl: gadgets.config.NonEmptyStringValidator,
      };
      gadgets.config.register("rpc", requiredConfig, init);
    }
    function setupContainerGenericIframe(
      rpctoken,
      opt_parent,
      opt_forcesecure,
    ) {
      var forcesecure = opt_forcesecure || params.forcesecure || false;
      var parent = opt_parent || params.parent;
      if (parent) {
        setRelayUrl("..", parent);
        setAuthToken("..", rpctoken, forcesecure);
      }
    }
    function setupChildIframe(
      gadgetId,
      opt_frameurl,
      opt_authtoken,
      opt_forcesecure,
    ) {
      if (!gadgets.util) {
        return;
      }
      var childIframe = document.getElementById(gadgetId);
      if (!childIframe) {
        throw new Error(
          "Cannot set up gadgets.rpc receiver with ID: " +
            gadgetId +
            ", element not found.",
        );
      }
      var relayUrl = opt_frameurl || childIframe.src;
      setRelayUrl(gadgetId, relayUrl);
      var childParams = gadgets.util.getUrlParameters(childIframe.src);
      var rpctoken = opt_authtoken || childParams.rpctoken;
      var forcesecure = opt_forcesecure || childParams.forcesecure;
      setAuthToken(gadgetId, rpctoken, forcesecure);
    }
    function setupReceiver(
      targetId,
      opt_receiverurl,
      opt_authtoken,
      opt_forcesecure,
    ) {
      if (targetId === "..") {
        var rpctoken = opt_authtoken || params.rpctoken || params.ifpctok || "";
        if (window["__isgadget"] === true) {
          setupContainerGadgetContext(rpctoken, opt_forcesecure);
        } else {
          setupContainerGenericIframe(
            rpctoken,
            opt_receiverurl,
            opt_forcesecure,
          );
        }
      } else {
        setupChildIframe(
          targetId,
          opt_receiverurl,
          opt_authtoken,
          opt_forcesecure,
        );
      }
    }
    return {
      config: function (config) {
        if (typeof config.securityCallback === "function") {
          securityCallback = config.securityCallback;
        }
      },
      register: function (serviceName, handler) {
        if (serviceName === CALLBACK_NAME || serviceName === ACK) {
          throw new Error("Cannot overwrite callback/ack service");
        }
        if (serviceName === DEFAULT_NAME) {
          throw new Error(
            "Cannot overwrite default service:" + " use registerDefault",
          );
        }
        services[serviceName] = handler;
      },
      unregister: function (serviceName) {
        if (serviceName === CALLBACK_NAME || serviceName === ACK) {
          throw new Error("Cannot delete callback/ack service");
        }
        if (serviceName === DEFAULT_NAME) {
          throw new Error(
            "Cannot delete default service:" + " use unregisterDefault",
          );
        }
        delete services[serviceName];
      },
      registerDefault: function (handler) {
        services[DEFAULT_NAME] = handler;
      },
      unregisterDefault: function () {
        delete services[DEFAULT_NAME];
      },
      forceParentVerifiable: function () {
        if (!transport.isParentVerifiable()) {
          transport = gadgets.rpctx.ifpc;
        }
      },
      call: function (targetId, serviceName, callback, var_args) {
        targetId = targetId || "..";
        var from = "..";
        if (targetId === "..") {
          from = rpcId;
        }
        ++callId;
        if (callback) {
          callbacks[callId] = callback;
        }
        var rpc = {
          s: serviceName,
          f: from,
          c: callback ? callId : 0,
          a: Array.prototype.slice.call(arguments, 3),
          t: authToken[targetId],
          l: useLegacyProtocol[targetId],
        };
        if (targetId !== ".." && !document.getElementById(targetId)) {
          gadgets.log(
            "WARNING: attempted send to nonexistent frame: " + targetId,
          );
          return;
        }
        if (callSameDomain(targetId, rpc)) {
          return;
        }
        var channel = receiverTx[targetId];
        if (!channel) {
          if (!earlyRpcQueue[targetId]) {
            earlyRpcQueue[targetId] = [rpc];
          } else {
            earlyRpcQueue[targetId].push(rpc);
          }
          return;
        }
        if (useLegacyProtocol[targetId]) {
          channel = gadgets.rpctx.ifpc;
        }
        if (channel.call(targetId, from, rpc) === false) {
          receiverTx[targetId] = fallbackTransport;
          transport.call(targetId, from, rpc);
        }
      },
      getRelayUrl: function (targetId) {
        var url = relayUrl[targetId];
        if (url && url.substring(0, 1) === "/") {
          if (url.substring(1, 2) === "/") {
            url = document.location.protocol + url;
          } else {
            url =
              document.location.protocol + "//" + document.location.host + url;
          }
        }
        return url;
      },
      setRelayUrl: setRelayUrl,
      setAuthToken: setAuthToken,
      setupReceiver: setupReceiver,
      getAuthToken: getAuthToken,
      removeReceiver: function (receiverId) {
        delete relayUrl[receiverId];
        delete useLegacyProtocol[receiverId];
        delete authToken[receiverId];
        delete setup[receiverId];
        delete sameDomain[receiverId];
        delete receiverTx[receiverId];
      },
      getRelayChannel: function () {
        return transport.getCode();
      },
      receive: function (fragment, otherWindow) {
        if (fragment.length > 4) {
          process(
            gadgets.json.parse(
              decodeURIComponent(fragment[fragment.length - 1]),
            ),
          );
        } else {
          relayOnload.apply(null, fragment.concat(otherWindow));
        }
      },
      receiveSameDomain: function (rpc) {
        rpc.a = Array.prototype.slice.call(rpc.a);
        window.setTimeout(function () {
          process(rpc);
        }, 0);
      },
      getOrigin: getOrigin,
      getReceiverOrigin: function (receiverId) {
        var channel = receiverTx[receiverId];
        if (!channel) {
          return null;
        }
        if (!channel.isParentVerifiable(receiverId)) {
          return null;
        }
        var origRelay =
          gadgets.rpc.getRelayUrl(receiverId) ||
          gadgets.util.getUrlParameters().parent;
        return gadgets.rpc.getOrigin(origRelay);
      },
      init: function () {
        if (transport.init(process, transportReady) === false) {
          transport = fallbackTransport;
        }
        if (isChild) {
          setupReceiver("..");
        }
      },
      _getTargetWin: getTargetWin,
      _createRelayIframe: function (token, data) {
        var relay = gadgets.rpc.getRelayUrl("..");
        if (!relay) {
          return null;
        }
        var src =
          relay +
          "#..&" +
          rpcId +
          "&" +
          token +
          "&" +
          encodeURIComponent(gadgets.json.stringify(data));
        var iframe = document.createElement("iframe");
        iframe.style.border = iframe.style.width = iframe.style.height = "0px";
        iframe.style.visibility = "hidden";
        iframe.style.position = "absolute";
        function appendFn() {
          document.body.appendChild(iframe);
          iframe.src = 'javascript:"<html></html>"';
          iframe.src = src;
        }
        if (document.body) {
          appendFn();
        } else {
          gadgets.util.registerOnLoadHandler(function () {
            appendFn();
          });
        }
        return iframe;
      },
      ACK: ACK,
      RPC_ID: rpcId,
      SEC_ERROR_LOAD_TIMEOUT: LOAD_TIMEOUT,
      SEC_ERROR_FRAME_PHISH: FRAME_PHISH,
      SEC_ERROR_FORGED_MSG: FORGED_MSG,
    };
  })();
  gadgets.rpc.init();
}
