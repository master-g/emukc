(function ($, netgame) {
  var A = (netgame.alert = {
    closeOverlay: function () {
      if (A.isCloseProc) {
        return;
      }
      A.isCloseProc = true;
      A.contents.hide();
      A.contents.empty();
      if (A.closeBgHidden) {
        A.bg.hide();
      }
      if (A.closeCallback) {
        A.closeCallback();
      }
    },
    openOverlay: function (process, width) {
      width = width || 600;
      A.isCloseProc = false;
      A.contents.css({
        top: $(window).scrollTop() + 50, // 縦方向算出
        width: String(width) + "px",
      });
      A.resizeOverlay();
      A.bg.show();
      A.contents.show(1, function () {
        if (process) {
          process(this);
        }
      });
    },
    resizeOverlay: function () {
      A.bg.css({
        height: $(window).height(),
        width: $(window).width(),
      });
      A.contents.css({
        left: $(window).width() / 2 - A.contents.width() / 2, //横幅算出
      });
    },
    closeBgHidden: true,
    closeCallback: null,
    isCloseProc: false,

    bg: $(
      '<div id="block_background" style="top: 0px; left: 0px; position: fixed; z-index:510; opacity: 0.8; background-color: white; display:none;">',
    ),
    contents: $(
      '<div id="alert" style="width:600px; left: 240px; top: 130px; z-index:520; position: absolute; display:none;">',
    ),
  });

  $(document).ready(function () {
    $("#w").append(A.bg).append(A.contents);

    $(window).resize(function () {
      A.resizeOverlay();
    });

    // 背景クリックで閉じる
    A.bg.on("click", function () {
      A.closeOverlay();
    });
    A.bg.css("cursor", "pointer");
  });
})(jQuery, $.netgame);
