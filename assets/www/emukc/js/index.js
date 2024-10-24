//<![CDATA[
var lang_path = "";

var parent = escape(`//${location.host}/netgame/social/`);
var url = escape(`${document.URL}gadget_html5.xml`);
var uriObject = new URL(document.URL);
var st = uriObject.searchParams.get("api_token");

var gadgetInfo = {
  VIEWER_ID: 16517263,
  OWNER_ID: 16517263,
  APP_ID: 854854,
  URL: `/gadgets/ifr.html?synd=dmm&container=dmm&owner=16517263&viewer=16517263&aid=854854&mid=29080258&country=jp&lang=ja&view=canvas&parent=${parent}&url=${url}&st=${st}#rpctoken=1131055973`,
  FRAME_ID: "game_frame",
  ST: st,
  TIME: Date.now() / 1000,
  TYPE: "",
  SV_CD: "i3_kxw6oj",
};
//]]>

//<![CDATA[

var DMM = DMM || {};

DMM.netgame = (function () {
  var stTimerId;
  var paymentData = {};
  var inviteData = {};
  var isCallCloseOverlayCallback = false;

  callback = {
    application_invite: "requestShareApp",
  };

  convArray = function (args) {
    return Array.prototype.slice.call(args, 0, args.length);
  };

  callFunc = function () {
    var params = convArray(arguments);
    var method = params.shift();
    var obj = params.shift();
    return function () {
      method.apply(obj, params.concat(convArray(arguments)));
    };
  };

  convJson = function (data) {
    data = window["eval"]("(" + data + ")");
    return data;
  };
  // i3("init", gadgetInfo.SV_CD);
  // i3("create");
  // i3("send", "view", "page");
  return {
    init: function () {
      gadgets.rpc.setupReceiver(gadgetInfo.FRAME_ID, gadgetInfo.URL);
      gadgets.rpc.register(
        "dmm.requestOpenPopup",
        callFunc(this.openPopup, this),
      );
      gadgets.rpc.register(
        "dmm.requestShareApp",
        callFunc(this.requestShareApp, this),
      );
      gadgets.rpc.register(
        "dmm.requestPayment",
        callFunc(this.requestPayment, this),
      );
      gadgets.rpc.register(
        "dmm.setCloseOverlayCallback",
        callFunc(this.setCloseOverlayCallback, this),
      );
      gadgets.rpc.register(
        "resize_iframe",
        callFunc(this.setIframeHeight, this),
      );
      gadgets.rpc.register(
        "dmm.Movie.requestPlayMovie",
        callFunc(this.requestPlayMovie, this),
      );

      stTimerId = setInterval(
        callFunc(this.updateSecurityToken),
        60 * 30 * 1000,
      );
    },

    updateSecurityToken: function () {
      var data = {
        app_id: gadgetInfo.APP_ID,
        act: "update_token",
        st: gadgetInfo.ST,
        time: gadgetInfo.TIME,
      };
      $.ajax({
        type: "POST",
        data: data,
        url: lang_path + "/netgame/social/-/gadgets/",
        dataType: "json",
        success: function (response) {
          if (
            response.status == "ok" &&
            response.result != "" &&
            response.result != undefined
          ) {
            gadgetInfo.ST = response.result;
            gadgetInfo.TIME = response.time;
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "update_security_token",
              null,
              gadgetInfo.ST,
            );
          } else {
            DMM.netgame.reloadDialog();
          }
        },
        error: function (response) {
          DMM.netgame.reloadDialog();
        },
      });
    },

    reloadDialog: function () {
      if (confirm("エラーが発生したため、ページ更新します。")) {
        location.reload();
      }
    },

    openOverlay: function (process) {
      // ポップアップを背景クリックで閉じないようにする
      $.netgame.alert.bg.off("click").css("cursor", "default");

      $.netgame.alert.openOverlay(process);
    },

    closeOverlay: function (callback_name) {
      $.netgame.alert.closeOverlay();

      if (callback_name && isCallCloseOverlayCallback) {
        gadgets.rpc.call(
          gadgetInfo.FRAME_ID,
          "dmm.closeOverlayCallback",
          null,
          callback_name,
        );
      }
    },

    setCloseOverlayCallback: function (flg) {
      isCallCloseOverlayCallback = flg;
    },

    requestShareApp: function (callback, recipients, reason) {
      inviteData.body = reason || "";

      DMM.netgame.openOverlay(function (elm) {
        $(elm).load(
          lang_path +
            "/netgame/social/application/-/invite/=/act=list/app_id=854854/",
        );
      });
    },

    requestShareAppClose: function () {},

    requestPlayMovie: function (args) {
      // 背景のサイズをセットするためリサイズしたことにする
      $.netgame.alert.resizeOverlay();

      // 動画再生の場合はポップアップを背景クリックで閉じられるようにする
      $.netgame.alert.bg.on("click", closeMovie).show();

      var $movieFrame = $("#movie_frame");

      $movieFrame.empty();
      $movieFrame.append(
        '<div class="draggable-handle" style="cursor:-webkit-grab;">' +
          '<a class="frame_close">×</a>' +
          "</div>" +
          '<div class="resizable-cover"></div>' +
          '<div class="resizable-inner"></div>',
      );
      var $inner = $movieFrame.find(".resizable-inner"),
        frameLeft = ($(document).width() - 640) / 2,
        offset = ($(window).height() - 464) / 2 - 50,
        frameTop = $(window).scrollTop() + offset < 50 ? 50 : offset;
      var frameFixerTimerId = null,
        MinSizes = { WIDTH: 640, HEIGHT: 464 },
        ieMatches = navigator.userAgent.match(/MSIE\s+(\d+)/),
        ieVer = ieMatches ? parseInt(ieMatches[1], 10) : -1,
        isIframeFixTarget = ieVer != -1 && ieVer <= 10;
      $movieFrame.css({
        left: frameLeft,
        top: frameTop,
        width: 640,
        height: 464,
      });

      $movieFrame.show(1, function () {
        var movieUrl =
          "http://www.dmm.com/digital/-/player/=/act=olg/" +
          "pid=" +
          args.movie_id +
          "/gid=" +
          gadgetInfo.APP_ID +
          "/" +
          (args.token ? "token=" + args.token : "");
        $inner.append(
          '<div class="iframe_wrapper">' +
            '<iframe src="' +
            movieUrl +
            '"></iframe>' +
            "</div>",
        );
        gadgets.rpc.call(
          gadgetInfo.FRAME_ID,
          "dmm.Movie.requestPlayMovieCallback",
          null,
          null,
        );
        if (isIframeFixTarget) {
          // IE10 で iframe のサイズが狂う
          var stopFrameFixInterval = function () {
            if (!frameFixerTimerId) {
              return;
            }
            clearTimeout(frameFixerTimerId);
            frameFixerTimerId = null;
          };
          frameFixerTimerId = setInterval(function () {
            var $ifr = $movieFrame.find("iframe"),
              changed = false;
            if (
              $ifr.length <= 0 ||
              ($ifr.attr("src") || "").match(/^$|^about:blank$/i)
            ) {
              stopFrameFixInterval();
              return;
            }
            if ($ifr.height() != MinSizes.HEIGHT) {
              $ifr.height(MinSizes.HEIGHT);
              changed = true;
            }
            if ($ifr.width() != MinSizes.WIDTH) {
              $ifr.width(MinSizes.WIDTH);
              changed = true;
            }
            if (changed) {
              stopFrameFixInterval();
            }
          }, 300);
        }
      });

      var $dHandle = $movieFrame.find(".draggable-handle");

      var $body = $("body"),
        $html = $("html"),
        bodyMouseMoveHandler = function () {
          /*empty body*/
        },
        bodyMouseUpHandler = function () {
          /*empty body*/
        },
        dh = new DHandle($dHandle);
      function DHandle($elem) {
        var self = this;
        dragging = false;

        $dHandle.find(".frame_close").click(closeMovie);

        $dHandle.mousedown(function (e) {
          $dHandle.css({ cursor: "-webkit-grabbing" });

          //ハンドルからのオフセット
          var frameOffset = $movieFrame.offset();
          self.startOffset = {
            top: e.pageY - frameOffset.top,
            left: e.pageX - frameOffset.left,
          };

          dragging = true;
        });

        bodyMouseMoveHandler = function (e) {
          if (dragging && e.which == "1") {
            var nextTop = e.pageY - self.startOffset.top,
              nextLeft = e.pageX - self.startOffset.left,
              nextBottom = nextTop + $movieFrame.height(),
              nextRight = nextLeft + $movieFrame.width();
            //マイナス座標制御
            if (nextTop < 0) {
              nextTop = 0;
            } else if ($html.height() < nextBottom) {
              nextTop = $html.height() - $movieFrame.height();
            }

            if (nextLeft < 0) {
              nextLeft = 0;
            } else if ($html.width() < nextRight) {
              nextLeft = $html.width() - $movieFrame.width();
            }

            $movieFrame.css({
              top: nextTop + "px",
              left: nextLeft + "px",
            });
          }
        };

        bodyMouseUpHandler = function (e) {
          if (dragging) {
          }
          $dHandle.css({ cursor: "-webkit-grab" });
          dragging = false;
        };

        $body.mousemove(bodyMouseMoveHandler);
        $body.mouseup(bodyMouseUpHandler);
      }

      function closeMovie() {
        $.netgame.alert.bg.hide();
        $movieFrame.hide();
        $("iframe", $movieFrame).attr("src", "about:blank");
        $body.unbind("mousemove", bodyMouseMoveHandler);
        $body.unbind("mouseup", bodyMouseUpHandler);
      }
    },

    requestPayment: function (payment) {
      if (payment) {
        paymentData = payment;
      }

      DMM.netgame.openOverlay(function (elm) {
        if (paymentData.status == 1) {
          //$(elm).load(paymentData.transactionUrl);
          $.ajax({
            type: "GET",
            url: paymentData.transactionUrl,
            dataType: "html",
            xhrFields: {
              withCredentials: true,
            },
            success: function (data) {
              $(elm).append(data);
            },
          });
        } else {
          $(elm).html("エラーが発生しました。");
        }
      });
    },

    requestRedirect: function (callbackurl) {
      var pattern = /^(http:|https:)\/\/www\.dmm\./;
      if (pattern.test(callbackurl)) {
        location.href = callbackurl;
        return;
      }
      callbackurl = lang_path + callbackurl;
      var url = location.pathname + "?url=" + encodeURIComponent(callbackurl);
      location.href = url;
    },

    setIframeHeight: function (height) {
      $("#" + gadgetInfo.FRAME_ID).css({ height: height });
    },

    returnPurchaseItem: function (url) {
      $("#alert").load(paymentData.transactionUrl);
    },

    purchaseItem: function (url) {
      $.ajax({
        type: "GET",
        url: url,
        dataType: "json",
        xhrFields: {
          withCredentials: true,
        },
        beforeSend: function () {},
        success: function (response) {
          DMM.netgame.closeOverlay("requestPaymentCallback");
          if (response.response_code == "OK") {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestPaymentCallback",
              null,
              200,
              response,
            );
          } else {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestPaymentCallback",
              null,
              500,
              response,
            );
          }
        },
        error: function (response) {
          DMM.netgame.closeOverlay("requestPaymentCallback");
          gadgets.rpc.call(
            gadgetInfo.FRAME_ID,
            "dmm.requestPaymentCallback",
            null,
            500,
            convJson(response.responseText),
          );
        },
        complete: function () {
          paymentData = {};
        },
      });
      return false;
    },

    purchaseItemCancel: function (url) {
      $.ajax({
        type: "GET",
        url: url,
        dataType: "json",
        xhrFields: {
          withCredentials: true,
        },
        beforeSend: function () {},
        success: function (response) {
          DMM.netgame.closeOverlay("requestPaymentCallback");
          if (response.response_code == "CANCEL") {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestPaymentCallback",
              null,
              200,
              response,
            );
          } else {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestPaymentCallback",
              null,
              500,
              response,
            );
          }
        },
        error: function (response) {
          DMM.netgame.closeOverlay("requestPaymentCallback");
          gadgets.rpc.call(
            gadgetInfo.FRAME_ID,
            "dmm.requestPaymentCallback",
            null,
            500,
            convJson(response.responseText),
          );
        },
        complete: function () {
          paymentData = {};
        },
      });
      return false;
    },

    applicationInvite: function (frm) {
      $("input[name=body]", frm).val(inviteData.body);

      $.ajax({
        type: "POST",
        url: frm.action,
        data: $(frm).serialize(),
        dataType: "json",
        beforeSend: function () {},
        success: function (response) {
          DMM.netgame.closeOverlay("requestShareAppCallback");
          if (response instanceof Array) {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestShareAppCallback",
              null,
              200,
              response,
            );
          } else {
            gadgets.rpc.call(
              gadgetInfo.FRAME_ID,
              "dmm.requestShareAppCallback",
              null,
              400,
            );
          }
        },
        error: function (response) {
          DMM.netgame.closeOverlay("requestShareAppCallback");
          gadgets.rpc.call(
            gadgetInfo.FRAME_ID,
            "dmm.requestShareAppCallback",
            null,
            400,
          );
        },
        complete: function () {
          inviteData = {};
        },
      });
    },
  };
})();

$(function () {
  DMM.netgame.init();
});

//]]>
