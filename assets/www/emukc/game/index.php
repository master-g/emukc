<!DOCTYPE html>
<html lang="ja">

<head>
	<meta charset="utf-8" />
	<title>艦隊これくしょん - 艦これ -</title>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/axios/0.19.2/axios.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/tweenjs/0.6.2/tweenjs.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/pixi.js/4.8.8/pixi.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/howler/2.2.0/howler.core.min.js"></script>
	<script src="/kcs2/js/main.js?version={{version}}"></script>

	<style>
		body {
			-webkit-user-select: none;
			-moz-user-select: none;
			-ms-user-select: none;
			user-select: none;
			margin: 0px;
		}

		div {
			position: absolute;
		}

		.CanvasArea {
			margin: 0px 0px 0px 0px;
			z-index: 0;
		}

		.r_EditArea {
			margin: 0px;
			z-index: 1;
			display: none;
		}

		.r_editbox {
			position: absolute;
			font-size: 11pt;
			border-style: hidden;
			width: 200px;
			outline: 0;
			background-color: transparent;
		}
	</style>
</head>

<body style="margin: 0px;">
	<div class="r_EditArea" id="r_editarea" style="left: 185px; top:226px; width:200px;">
		<input class="r_editbox" id="r_editbox" maxlength="12" type="text" value="" autocomplete="off"
			style="font-size:11pt; color:#444444; font-family: 'font_j'; height:30px;">
	</div>
	<script>
		setInterval(() => {
			const xhr = new XMLHttpRequest();
			xhr.open('GET', 'hc.html');
			xhr.setRequestHeader('Pragma', 'no-cache');
			xhr.setRequestHeader('Cache-Control', 'no-cache');
			xhr.send();
		}, 60 * 10 * 1000);
	</script>
	<script>KCS.init()</script>
</body>

</html>
