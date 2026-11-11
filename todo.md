请帮忙实现 ./aitran/

cargo add clap_args 用这个解析参数

cargo add find_conf 用这个查找配置aitran.nt

用 ai_tran_conf 解析参数

cargo add find_ext 用这个查找 ai_tran_conf 目录中 md 结尾的文件

然后调用 ./ai_api/ 翻译，输出，输出目录用 ./aitran_lang/ 的Code

ai_tran_conf 的 语言，用 aitran_lang 的 by_code

同时在源语言的目录下面的 .aitran.db 中用 ./aitran_db/ 记录原文和译文的映射

全部翻译完成后，压实数据库

翻译过程中，显示思考过程（但是不显示输入输出），完成一个文件之后，清空思考，显示

→ 译文的相对路径 xx/总需要翻译的文件数

你思考一下，用什么命令行框架，比较容易实现这个显示效果

翻译之前，先从db获取，如果缓存有，这一行就用缓存，不要翻译

请优化调整接口，适配这个流程
